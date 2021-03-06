extern crate nalgebra as na;
extern crate ncollide;

use na::{Point3, Vector3, Isometry3};
use ncollide::partitioning::BVT;
use ncollide::shape::{Cone, Ball, Cuboid, Capsule};
use ncollide::query::{RayInterferencesCollector, RayCast, Ray};
use ncollide::bounding_volume::{self, BoundingSphere, HasBoundingVolume};

/*
 * Custom trait to group `HasBoudingSphere` and `RayCast` together.
 */
trait Shape3: HasBoundingVolume<Isometry3<f64>, BoundingSphere<Point3<f64>>> +
              RayCast<Point3<f64>, Isometry3<f64>> {
}

impl<T> Shape3 for T
    where T: HasBoundingVolume<Isometry3<f64>, BoundingSphere<Point3<f64>>> +
             RayCast<Point3<f64>, Isometry3<f64>> {
}

fn main() {
    let ball = Ball::new(0.5);
    let caps = Capsule::new(0.5, 0.75);
    let cone = Cone::new(0.5, 0.75);
    let cube = Cuboid::new(Vector3::new(1.0, 0.5, 1.0));

    let shapes = [
        &ball as &Shape3,
        &caps as &Shape3,
        &cone as &Shape3,
        &cube as &Shape3
    ];

    let poss = [
        Isometry3::new(Vector3::new(0.0, 0.0, 1.0), na::zero()),
        Isometry3::new(Vector3::new(0.0, 0.0, 2.0), na::zero()),
        Isometry3::new(Vector3::new(0.0, 0.0, 3.0), na::zero()),
        Isometry3::new(Vector3::new(0.0, 2.0, 4.0), na::zero())
    ];

    let idx_and_bounding_spheres: Vec<(usize, BoundingSphere<Point3<f64>>)> = vec!(
        (0usize, bounding_volume::bounding_sphere::<Point3<f64>, _, _>(shapes[0], &poss[0])),
        (1usize, bounding_volume::bounding_sphere(shapes[1], &poss[1])),
        (2usize, bounding_volume::bounding_sphere(shapes[2], &poss[2])),
        (3usize, bounding_volume::bounding_sphere(shapes[3], &poss[3]))
    );

    let bvt      = BVT::new_balanced(idx_and_bounding_spheres);
    let ray_hit  = Ray::new(na::origin(), Vector3::z());
    let ray_miss = Ray::new(na::origin(), -Vector3::z());

    /*
     * Ray cast using a visitor.
     */
    let mut collector_hit:  Vec<usize> = Vec::new();
    let mut collector_miss: Vec<usize> = Vec::new();

    // We need a new scope here to avoid borrowing issues.
    {
        let mut visitor_hit  = RayInterferencesCollector::new(&ray_hit, &mut collector_hit);
        let mut visitor_miss = RayInterferencesCollector::new(&ray_miss, &mut collector_miss);

        bvt.visit(&mut visitor_hit);
        bvt.visit(&mut visitor_miss);
    }

    assert!(collector_hit.len()  == 3);
    assert!(collector_miss.len() == 0);
}

