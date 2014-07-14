use geom::Cone;
use volumetric::Volumetric;
use math::{Scalar, Vect, AngularInertia};

#[dim2]
use nalgebra::na::Indexable;
#[dim2]
use nalgebra::na;


#[dim3]
use std::num::Float;
#[dim3]
use nalgebra::na::Indexable;
#[dim3]
use nalgebra::na;

/// Computes the volume of a cone.
#[dim2]
#[inline]
pub fn cone_volume(half_height: &Scalar, radius: &Scalar) -> Scalar {
    // same as a isosceles triangle
    *radius * *half_height * na::cast(2.0f64)
}

/// Computes the volume of a cone.
#[dim3]
#[inline]
pub fn cone_volume(half_height: &Scalar, radius: &Scalar) -> Scalar {
    *radius * *radius * Float::pi() * *half_height * na::cast(2.0f64 / 3.0)
}

/// Not yet implemented in 4d.
#[dim4]
#[inline]
pub fn cone_volume(_: &Scalar, _: &Scalar) -> Scalar {
    fail!("Not yet impelmented in 4d.")
}

#[dim2]
impl Volumetric for Cone {
    fn mass_properties(&self, density: &Scalar) -> (Scalar, Vect, AngularInertia) {
        let mass = cone_volume(&self.half_height(), &self.radius()) * *density;

        // FIXME: not sure about that…
        let mut res: AngularInertia = na::zero();

        res.set(
            (0, 0),
            self.radius() * self.half_height() * self.half_height() * self.half_height()
            / na::cast(3.0f64)
            );

        let mut center: Vect = na::zero();
        center.set(1, -self.half_height() / na::cast(2.0f64));

        (mass, center, res)
    }
}

#[dim3]
impl Volumetric for Cone {
    fn mass_properties(&self, density: &Scalar) -> (Scalar, Vect, AngularInertia) {
        let mass        = cone_volume(&self.half_height(), &self.radius()) * *density;
        let m_sq_radius = mass * self.radius() * self.radius();
        let m_sq_height = mass * self.half_height() * self.half_height() *
                          na::cast(4.0f64);
        let off_principal = m_sq_radius * na::cast(3.0f64 / 20.0) +
                            m_sq_height * na::cast(3.0f64 / 5.0);

        let principal = m_sq_radius * na::cast(3.0f64 / 10.0);

        let mut res: AngularInertia = na::zero();

        res.set((0, 0), off_principal.clone());
        res.set((1, 1), principal);
        res.set((2, 2), off_principal);

        let mut center: Vect = na::zero();
        center.set(1, -self.half_height() / na::cast(2.0f64));

        (mass, center, res)
    }
}

#[dim4]
impl Volumetric for Cone {
    fn mass_properties(&self, _: &Scalar) -> (Scalar, Vect, AngularInertia) {
        fail!("mass_properties is not yet implemented for cones.")
    }
}
