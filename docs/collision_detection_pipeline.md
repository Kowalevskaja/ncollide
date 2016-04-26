# Collision detection pipeline

While detecting proximities and computing contacts between only two objects
might be useful to compute their immediate interactions, we often work with
complex scenes involving thousands of objects. Iterating through each pair of
object and testing each of them for intersection is a $\mathcal{O}(n^2)$
process where $n$ is the number of objects. This is obviously not practicable
in real-time.


To overcome this issue, **ncollide** implements a collision detection pipeline
decomposed into two stages: the [broad phase](#broad-phase) and the [narrow
phase](#narrow-phase). The broad phase has a global knowledge of the position
of every object simultaneously so it can use spacial partitioning with
conservative interference detection algorithms to find all the potential
collision pairs very efficiently: $\mathcal{O}(n \log(n))$ time in average or
even $\mathcal{O}(1)$ if time coherence is high. Then the narrow phase iterates
on those pairs individually and performs the exact geometric query.  Note that
some objects paired by the broad phase may not actually be in contact (_false
positives_) as it only performs approximate tests. On the other hand, a broad
phase is guaranteed not to produce any _false negative_: two interfering
objects are always guaranteed to be paired and reported to the narrow phase.


# Broad phase

A broad phase on **ncollide** must implement the `broad_phase::BroadPhase`
trait that ensures that objects can be added, removed, updated, and supports
common geometric queries:

| Method                 | Description                                     |
|--                      | --                                              |
| `.deferred_add(uid, bv, data)`                    | Informs the broad phase algorithm that a new object with the identifier `uid`, the bounding volume `bv`, and the associated data `data` has to be added during the next update. |
| `.deferred_remove(uid)`                           | Informs the broad phase algorithm that the object identified by `uid` must be removed at the next update. |
| `.deferred_set_bounding_volume(uid, bv)`          | Informs the broad phase algorithm that the object identified by `uid`’s bounding volume has to be replaced by `bv` at the next update. |
| `.deferred_recompute_all_proximities()` | Forces the broad phase to recompute and re-report all collision pairs at the next update. |
| `.update(filter, callback)`                       | Updates this broad phase algorithm, actually performing pending object additions and removals. `filter` is a predicate that indicates if a new potential collision pair is valid. If it returns `true` for a given pair, `callback` will be called for it. |
| `.interferences_with_bounding_volume(bv, result)` | Fills `result` with references to each object which bounding volume intersects the bounding volume `bv`. |
| `.interferences_with_ray(ray, result)`            | Fills `result` with references to each object which bounding volume intersects the ray `ray`. |
| `.interferences_with_point(point, result)`        | Fills `result` with references to each object which bounding volume contains the point `point`. |

Let us clarify what _associated data_ means here. A broad phase is guaranteed
to associate some pieces of data to each object. Those data are completely
user-defined (e.g. they can even be as general as `Box<Any>`) and are passed as
argument to the user-defined callbacks when the `update` method is called.
Therefore, feel free to store in there any piece of data that may be useful to
identify the object on your side.

Note that methods with names prefixed by `deferred_` have no effect until the
next call to `.update(...)`. This allows the broad phase to perform one global
update even if several objects are moved individually. This update relies on a
collision filter and a callback. First, the filter should always have results
constant wrt. time. _Constant_ means that if at some time the filter returns
`true` (resp. `false`) for some object pair $(\mathcal{A}, \mathcal{B})$, it is
expected to always return `true` (resp. `false`) for $(\mathcal{A},
\mathcal{B})$ at any time in the future as well. If the filter changes at some
point (hence breaking this constancy), the method
`.deferred_recompute_all_proximities()` must be called in order to inform the
broad phase that it should be re-executed on all potential collision pairs
already detected. The second closure `callback` passed at update-time is the
bridge between the broad phase and the narrow phase: it will be called on each
potential collision pair that has not been filtered out.

Finally, a broad phase algorithm often being inherently incremental, keep in
mind that `callback` will usually be called only once on each new potential
contact pair created or removed as a consequence of filter change or objects
being moved. Pairs unaffected by recent changes will usually **not** be
re-reported.

### The DBVT broad phase

The `broad_phase::DBVTBroadPhase` is based on a Dynamic Bounding Volume Tree
(DBVT) to detect interferences. It implements the `BroadPhase` trait described
above.  The `partitioning::DBVT` structure itself is a proper binary tree that
maps a bounding volume to the object it bounds on its leaves. Internal nodes
only contain a bounding volume that spatially bounds all the leaves of the
sub-tree it is root of. The following figure shows an example of tree that
contains a set of brown objects with their red AABB. Note that, instead of the
exact bounding volumes (red), the `DBVTBroadPhase` stores their loosened
version (black):

<center>
![dbvt](../img/AABB_tree_DBVT.svg)
</center>

Creating an empty `DBVTBroadPhase` is simple using the `::new(margin,
small_keys)` constructor:
```rust
let mut dbvt = DBVTBroadPhase::new(0.02, false);
```
Storing the loosened bounding volumes instead of the exact ones is a
significant optimization for scenes where the broad phase has to track contact
pairs involving slow-moving objects: the DBVT is modified by the broad phase
only when the displacement of an object is large enough to make its exact
bounding volume move out of the loosened version stored on the tree. That way
objects moving at high frequency but low amplitude will almost never trigger an
update, at the cost of slightly less tight bounding volumes for interference
detection. The amount of loosening is controlled by the first constructor
argument `margin`. The second argument `small_keys` is here for optimization
purpose.  Set it to `true` if and only if you know that the integer keys you
use to identify your objects are small (as in "small enough for them to be used
as keys on a `Vec` instead of a `HashMap`"). If you are not sure of the values
your keys may take, set `small_keys` to `false`.


The following example creates four balls, adds them to a `DBVTBroadPhase`,
updates the broad phase, and removes some of them.

#### 2D example <div class="d2" onclick="window.open('https://raw.githubusercontent.com/sebcrozet/ncollide/master/examples/dbvt_broad_phase2d.rs')" ></div>
```rust
/*
 * Create the objects.
 */
let poss = [ Isometry2::new(Vector2::new(0.0, 0.0), na::zero()),
             Isometry2::new(Vector2::new(0.0, 0.5), na::zero()),
             Isometry2::new(Vector2::new(0.5, 0.0), na::zero()),
             Isometry2::new(Vector2::new(0.5, 0.5), na::zero()) ];

// We will use the same geometry for the four objects.
let ball = Ball::new(0.5);

/*
 * Create the broad phase.
 * We know we will use small uids (from 0 to 3)so we can pass `true` as the second argument.
 */
let mut bf = DBVTBroadPhase::new(0.2, true);

// First parameter:  a unique id for each object.
// Second parameter: the object bounding box.
// Third parameter:  some data (here, the id that identify each object).
bf.deferred_add(0, bounding_volume::aabb(&ball, &poss[0]), 0);
bf.deferred_add(1, bounding_volume::aabb(&ball, &poss[1]), 1);
bf.deferred_add(2, bounding_volume::aabb(&ball, &poss[2]), 2);
bf.deferred_add(3, bounding_volume::aabb(&ball, &poss[3]), 3);

// Update the broad phase.
// The collision filter (first closure) prevents self-collision.
bf.update(&mut |a, b| *a != *b, &mut |_, _, _| { });

assert!(bf.num_interferences() == 6);

// Remove two objects.
bf.deferred_remove(0);
bf.deferred_remove(1);

// Update the broad phase.
// The collision filter (first closure) prevents self-collision.
bf.update(&mut |a ,b| *a != *b, &mut |_, _, _| { });

assert!(bf.num_interferences() == 1)
```

#### 3D example <div class="d3" onclick="window.open('https://raw.githubusercontent.com/sebcrozet/ncollide/master/examples/dbvt_broad_phase3d.rs')" ></div>
```rust
/*
 * Create the objects.
 */
let poss = [ Isometry3::new(Vector3::new(0.0, 0.0, 0.0), na::zero()),
             Isometry3::new(Vector3::new(0.0, 0.5, 0.0), na::zero()),
             Isometry3::new(Vector3::new(0.5, 0.0, 0.0), na::zero()),
             Isometry3::new(Vector3::new(0.5, 0.5, 0.0), na::zero()) ];

// We will use the same geometry for the four objects.
let ball = Ball::new(0.5);

/*
 * Create the broad phase.
 * We know we will use small uids (from 0 to 3)so we can pass `true` as the second argument.
 */
let mut bf = DBVTBroadPhase::new(0.2, true);

// First parameter:  a unique id for each object.
// Second parameter: the object bounding box.
// Third parameter:  some data (here, the id that identify each object).
bf.deferred_add(0, bounding_volume::aabb(&ball, &poss[0]), 0);
bf.deferred_add(1, bounding_volume::aabb(&ball, &poss[1]), 1);
bf.deferred_add(2, bounding_volume::aabb(&ball, &poss[2]), 2);
bf.deferred_add(3, bounding_volume::aabb(&ball, &poss[3]), 3);

// Update the broad phase.
// The collision filter (first closure) prevents self-collision.
bf.update(&mut |a, b| *a != *b, &mut |_, _, _| { });

assert!(bf.num_interferences() == 6);

// Remove two objects.
bf.deferred_remove(0);
bf.deferred_remove(1);

// Update the broad phase.
// The collision filter (first closure) prevents self-collision.
bf.update(&mut |a ,b| *a != *b, &mut |_, _, _| { });

assert!(bf.num_interferences() == 1)
```


# Narrow phase
As the broad phase detects pairs of objects that may potentially interact, they
can be passed to the narrow phase which will instantiate the persistent
algorithms to perform exact proximity detection or contact point computation.
The narrow phase is also responsible for notifying the user when an interaction
(proximity, contact, etc.) starts or stops. Every narrow phase must implement
the `narrow_phase::NarrowPhase` trait.

| Method | Description |
|--      | --          |
| `.update(...)` | Updates the narrow phase actually performing contact and proximity computation. |
| `.handle_interaction(..., objs, fk1, fk2, started)` | Tells the narrow phase that the objects given by `objs[fk1]` and `objs[fk2]` start or stop interacting. |
| `.contact_pairs(objects)` | Returns all the contact pairs. |
| `.proximity_pairs(objects)` | Returns all the proximity pairs. |

The `.handle_interaction(...)` method will instantiate the correct persistent
contact or proximity detection algorithm for the given pair of objects. It will
also free those that are no longer relevant, e.g., because the `started` flags
is set to `false`. It will usually not perform any actual geometric query as
this is the task of the `.update(...)` method. Both methods take two _signals_
as arguments: a `ContactSignal` and a `ProximitySignal`. Both are sets of
callbacks that are called when a contact or proximity starts or stops. Hence,
the user may add their own contact and proximity event handlers to those.

The `narrow_phase::DefaultNarrowPhase` is the default implementation of the
narrow phase and should be suitable for most applications.

## Persistent proximity detection
Persistent proximity detection algorithms differ from the
`query::proximity(...)` [function](../geometric_queries/#proximity) in that the
former require a structure to be instantiated. This structure can then be
re-used over time with the same shapes but with different positions and
proximity margins. This allows the proximity detector to perform significant
optimizations if the positions change slowly over time − this is usually called
_temporal coherence_. A persistent proximity detector must implement the
`narrow_phase::ProximityDetector` trait.

| Method | Description |
|--      | --          |
| `.update(dispatcher, ma, a, mb, b, margin)` | Actually performs the proximity determination between the objects `a` and `b` respectively transformed by `ma` and `mb`. |
| `.proximity(&self)` | Returns the result of the last update. |

Note that for a given proximity dector instance, the shapes `a` and `b` are
assumed never to change over time. Changing them may lead to unexpected
results. The `margin` argument has the same semantic as with the
`query::proximity(...)` [function](../geometric_queries/#proximity). The
`dispatcher` argument is a trait-object that is responsible for instantiating
the correct proximity algorithm for a given pair of shapes. This is useful when
the proximity determination algorithm is recursive, e.g., for [composite
shapes](../geometric_representations/#composite-shapes). This trait-object of
type `narrow_phase::ProximityDispatcher` has only one method that may return
`None` if no algorithm is known for the given shapes:

| Method                           | Description |
|--                                | --          |
| `.get_proximity_algorithm(a, b)` | Returns the persistent proximity determination algorithm dedicated to the shapes `a` and `b`. |

The `narrow_phase::ProximityAlgorithm` return type is just an alias for a boxed
`ProximityDetector` trait-object.

Note that if you are not interested by the whole collision detection pipeline
but only by the persistent proximity determination algorithms, the
`narrow_phase::DefaultProximityDispatcher` can be used alone to retrieve the correct
proximity detection algorithm, depending on your geometries:

```rust
let dispatcher = DefaultProximityDispatcher2::new();
let shape1 = Ball::new(0.5);
let shape2 = Cylinder::new(0.5, 1.0);
let shape3 = Cone::new(0.5, 1.0);

let ball_vs_cylinder_detector = dispatcher.get_proximity_algorithm(&shape1, &shape2);
let ball_vs_cone_detector     = dispatcher.get_proximity_algorithm(&shape1, &shape3);
let cylinder_vs_cone_detector = dispatcher.get_proximity_algorithm(&shape2, &shape3);
```

## Persistent contact generation

Persistent contact generation follow the same logic as persistent proximity
detection, but for  multiple contact points computation. Such collision
detector the `narrow_phase::CollisionDetector` trait.

| Method | Description |
|--      | --          |
| `.update(...)`  | Given the two shapes and their position, determines their contact geometry without returning it. |
| `.num_colls() ` | The number of contacts generated by the last update.  |
| `.colls(out)`   | Collects to `out` the contacts generated by the last update. |

Just like the proximity detectors, the `.update(...)` method requires a
dispatcher which is a trait-object of type `CollisionDispatcher` with one
method that may return `None` if no algorithm is known for the given shapes:

| Method                           | Description |
|--                                | --          |
| `.get_collision_algorithm(a, b)` | Returns the persistent contact computation algorithm dedicated to the shapes `a` and `b`. |

The `narrow_phase::CollisionAlgorithm` return type is just an alias for a boxed
`CollisionDetector` trait-object.

If you are not interested by the whole collision detection pipeline but only by
the persistent contact determination algorithms, the
`narrow_phase::DefaultCollisionDispatcher` can be used alone to retrieve the
correct collision detection algorithm, depending on your geometries:

```rust
let dispatcher = DefaultCollisionDispatcher2::new();
let shape1 = Ball::new(0.5);
let shape2 = Cylinder::new(0.5, 1.0);
let shape3 = Cone::new(0.5, 1.0);

let ball_vs_cylinder_detector = dispatcher.get_collision_algorithm(&shape1, &shape2);
let ball_vs_cone_detector     = dispatcher.get_collision_algorithm(&shape1, &shape3);
let cylinder_vs_cone_detector = dispatcher.get_collision_algorithm(&shape2, &shape3);
```

### Conforming contacts
Most specific algorithms implemented on **ncollide** are limited to punctual
contacts. Therefore if you have a cube lying on a plane, only one contact will
be returned (left) instead of a theoretically infinite set of point (right):

<center>
![](../img/conforming_contact.svg)
</center>


Those planar contacts that cannot be assimilated to a single point are called
_conforming contacts_. Approximating the shape of such contact with more than
one point is critical for, e.g., physics simulation applications with contact
laws based exclusively on a discrete number of isolated contact points. The
following figure shows what would happen on this kind of physics simulation
with a conforming contact approximated by only one point:

<center>
![](../img/contact.svg)
</center>

Here, the cube is falling toward the plane. When a contact is detected, the
cube is penetrating the plane and the physics engine will try to correct this
situation by applying a force to the contact point. This forces the object to
(unrealistically) rotate, moving the contact point to the over side.  This
alternation between two contact points due to unwanted rotations makes the
simulation instable and unrealistic. That is why **ncollide** provides contact
determination algorithms wrappers that can generate a full contact manifold
either incrementally or in a single shot.

### Incremental contact manifold generation
The `narrow_phase::IncrementalContactManifoldGenerator` stores the results of the
wrapped collision detector, and updates them as the objects involved in the
contact move. After a few updates, a full manifold will be created:

<center>
![](../img/acc_contact.svg)
</center>

Obviously, storing the contact points at each update indefinitely is costly
and not necessary. This is why if more than 2 (resp. 4) contacts in 2D (resp.
3D) are stored, the surplus will be removed. This removal is smart enough to
maximize the area spanned by the remaining contact points. For example, the red
contact has been removed from the last part of the previous figure because it
is less significant than the two others.

The `IncrementalContactManifoldGenerator` can be used to wrap any structure
that implements the `CollisionDetector` trait and that generates a single
contact point:

```rust
let single_point_generator  = PlaneSupportMapCollisionDetector2::new();
let full_manifold_generator = IncrementalContactManifoldGenerator::new(plane_vs_support_map);
```

### One-shot contact manifold generation
The `narrow_phase::OneShotContactManifoldGenerator` is slower but more accurate
than the incremental manifold generator as it is able to generate a full
manifold as soon as the first contact is detected. This is done by
virtually applying small rotations to one of the object in contact and
accumulating the new points generated by the wrapped collision detector.
Therefore, from the user point of view, a full manifold has been generated in
only one step:
<center>
![](../img/one_shot_contact.svg)
</center>

To reduce the computation times of this wrapper, the
`OneShotContactManifoldGenerator` stops this rotation-based algorithm and acts
like the incremental one as long as the manifold has been generated once.

The `OneShotContactManifoldGenerator` can be used to wrap any structure
that implements the `CollisionDetector` trait and that generates a single
contact point:

```rust
let single_point_generator  = PlaneSupportMapCollisionDetector2::new();
let full_manifold_generator = OneShotContactManifoldGenerator::new(plane_vs_support_map);
```

# Collision world 
The `world::CollisionWorld` is expected to be the main interface between the
user and its geometrical scene. It groups:

* Geometrical shapes and their positions using so-called _collision objects_.
* A broad phase set to the `DBVTBroadPhase` by default.
* A narrow phase set to the  `DefaultNarrowPhase` by default.

All those are hidden between a high-level interface so that the user does not
have to manually modify and synchronize the two collision detection stages.  An
empty collision world is created with the constructor
`CollisionWorld::new(margin, small_uids)`. Both arguments are only for
optimization purpose and have the same semantic as their
[counterparts](#the-dbvt-broad-phase) for the creation of the DBVT broad-phase:
the `margin` is the amount of loosening for each bounding volume (this does not
affect the geometric shapes). While this value is depends on your specific
application, a value of 0.02 is usually good enough if your objects have an
average size of 1 (no matter which
[units](../faq/#which-units-are-used-by-ncollide) you use). The `small_uids`
flag indicates if your collision object identifiers will be small enough to be
indices of a `Vec` instead of a `HashMap`. It should be set to `false` if you
are not sure.

## Collision objects
The `world::CollisionObject` structures are the main citizens of the collision
world. They contain all information needed to describe an object and its
position in space.

| Field               | Description                                                         |
|--                   | --                                                                  |
| `.position`         | The collision object position in space.                             |
| `.shape`            | The geometrical shape on the collision object.                      |
| `.collision_groups` | Groups used to prevent interactions with some other objects. |
| `.query_type`       | The kind of query this object is to be involved in.                 |
| `.data`             | User-defined data associated to this object. This will never be modified by **ncollide**. |

The two fields `.collision_groups` and `.query_type` affect how the object will
interact with the others on the collision world and are detailed in subsequent
sections.

A collision object should not be created directly by the user. Instead it
is initialized internally by the collision world with the
`CollisionWorld::add(uid, ...)` method. Its first argument is a unique
identifier of your choice that allows you to update this collision object
later. The `CollisionObject` instance created by the world can be retrieved
with the `.collision_object(uid)` method. Currently, a collision object can
only be moved or removed from the collision world. Future versions of
**ncollide** will allow you to modify its shape, collision groups and query
types as well. All those modifications will wait until the next call to
`.update()` to be actually performed efficiently:

| Field                              | Description |
|--                                  | --          |
| `.deferred_set_position(uid, pos)` | Sets the position of the collision object identified by `uid` at the next update. |
| `.deferred_remove(uid)`            | Removes the collision object identified by `uid` at the next update. |

Adding an object with the same `uid` as another one already present on the
collision world will panic. Therefore, keep in mind that because object removal
is deferred until the next update it is necessary to wait for the next call to
`.update()` in order to be able to re-use a removed object's identifier. For
example, the following (pseudo-code) panics:

```rust
collision_world.add(0, ...);
collision_world.update();
collision_world.deferred_remove(0);

collision_world.add(0, ...); // Panic! The uid 0 is already present.
```

While the following will work as expected:

```rust
collision_world.add(0, ...);
collision_world.update();
collision_world.deferred_remove(0);

collision_world.update();    // Actually remove 0 ...
collision_world.add(0, ...); // ... so it can be re-used for another object.
```

### Collision groups
Collision groups are the main way to prevent some object from interacting with
each other. Internally, it is represented as a few bit fields so verifying if
two object can interact is only a matter of performing bitwise operations. The
`world::CollisionGroups` structure defined three masks:

1. The **membership** mask − the list of groups this collision object is part
   of.
2. The **whitelist** mask − the list of groups this collision object can
   interact with if it is not blacklisted.
3. The **blacklist** mask − the list of groups this collision object must never
   interact with.

Note that the blacklist has precedence over the whitelist, i.e., if a group is
on both lists, interactions will be forbidden. Setting up a `CollisionGroups`
structure must follow the following reasoning: each object can be member of
several group. If it encounters another object, a geometric query will be
performed by the narrow phase if and only if:

* Each object is member of **at least one** group part of the other object's whitelist.
* **None** of the object is member of any group part of the other object's
  blacklist.

A collision group is identified by a positive number on the range $[\![ 0, 29 ]\!]$.
Thus, only 30 groups are available. By default, the `CollisionGroups::new()`
constructor will initialize the masks such that the object is member of all
groups, whitelists all groups, and blacklists none of them. You may modify,
set, or copy those masks with the corresponding methods prefixed by `.modify_`,
`.set_`, and `.copy_`.

Finally, there exists one special group for self-collision. Because this is
meaningful only for deformable shapes (which are not yet explicitly supported
by **ncollide**), it is disabled by default. It may be enabled with the
`.enable_self_collision()` method. This will allow the narrow phase to perform
geometric queries involving the same collision object twice.

#### Example <div class="btn-primary" onclick="window.open('https://raw.githubusercontent.com/sebcrozet/ncollide/master/examples/collision_groups.rs')"></div>

In the following example, we have three `CollisionGroups` A, B, and C. The
first one is such that:

* It is member of the groups 1, 3, and 6.
* It whitelists the groups 6 and 7.
* It blacklists the group 1.

The second one is such that:

* It is member of the groups 1, 3, and 7.
* It whitelists the groups 3 and 7.
* It does not blacklist anything.

The third one is such that:

* It is member of the groups 6 and 9.
* It whitelists the groups 3 and 7.
* It does not blacklist anything.

As a result, we observe that:

* A and B will **not** interact because B is part of the group 1 which is
  blacklisted by A.
* B and C will **not** interact either because, even if C whitelists the group
  3 (which B is part of), B does not whitelists neither the group 6 nor the
  group 9 (which B is part of).
* A and C **will** interact because A whitelists the group 6 (which C is part
  of), and, reciprocally, C whitelists the group 3 (which A is part of).
  In addition, C is not part of the group 1 which is blacklisted by A.

```rust
let mut a = CollisionGroups::new();
let mut b = CollisionGroups::new();
let mut c = CollisionGroups::new();

a.set_membership(&[1, 3, 6]);
a.set_whitelist(&[6, 7]);
a.set_blacklist(&[1]);

b.set_membership(&[1, 3, 7]);
b.set_whitelist(&[3, 7]);

c.set_membership(&[6, 9]);
c.set_whitelist(&[3, 7]);

assert!(!a.can_interact_with_groups(&b));
assert!(!b.can_interact_with_groups(&c));
assert!(a.can_interact_with_groups(&c));
```

### Query type
The query type indicates which kind of geometric query should be executed by
the narrow phase on this object. Two choices are given by the
`world::CollisionQueryType` enumeration. More query types for, e.g., minimal
distance computation only, may be added in the future:

1. `::Contacts(prediction)` − two objects with this query
   type will have their [contact points](../geometric_queries/#contact)
   computed. Contact points will be generated as long as the two objects are
   penetrating or closer than the sum of both `prediction` values.
2. `::Proximity(margin)` − if at least one object has this query type, then
   only [proximity detection](../geometric_queries/#proximity) will be
   performed. Shapes separated by a distance larger than the sum of their
   `margin` will be considered disjoint.

Note that if the two shapes request different query types, only the simplest is
performed. For example, one shape having a `::Contact(prediction1)` query type
interacting with a shape with a `::Proximity(margin2)` query type will generate
only a proximity query. In other words, the contact query type is implicitly
reinterpreted as a `::Proximity(prediction1)` query type.

## World-scale geometric queries
Because is groups collision object with efficient acceleration data structure,
it is natural that the collision world allows the user to apply
[single-shape](./geometric_queries/#single-shape-queries) geometric queries to
all the objects at the same time:

| Method                 | Description |
|--                      | --          |
| `.interferences_with_ray(ray, groups)` | Returns an iterator through all objects able to interact with `groups` and intersecting `ray`. |
| `.interferences_with_point(point, groups)` | Returns an iterator through all objects able to interact with `groups` and containing `point`. |
| `.interferences_with_aabb(aabb, groups)` | Returns an iterator through all object that can interact with `groups` and with an AABB that intersects `aabb`.  |

Notice that not all collision object have to be affected by the geometric
query: only those with a collision groups compatible with `groups` will
actually perform the actual query. The results of pairwise geometric queries
performed by the narrow phase can be retrieved as well through the collision
world:

| Method                | Description |
|--                     | --          |
| `.contact_pairs()`    | An iterator through the contact pairs created by the narrow phase.   |
| `.proximity_pairs()`  | An iterator through the proximity pairs created by the narrow phase. |
| `.contacts()`         | An iterator through the contacts computed by the narrow phase.       |

## Custom filters and callbacks

## Example