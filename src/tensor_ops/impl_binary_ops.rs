use crate::prelude::*;
use std::ops::{Add, Mul, Sub};

fn matmul<const M: usize, const N: usize, const O: usize>(
    x: &[[f32; N]; M],
    y: &[[f32; O]; N],
) -> [[f32; O]; M] {
    let mut result = [[0.0; O]; M];
    for m in 0..M {
        for n in 0..N {
            let x_mn = &x[m][n];
            for o in 0..O {
                result[m][o] += x_mn * &y[n][o];
            }
        }
    }
    result
}

fn transpose<const M: usize, const N: usize>(x: &[[f32; N]; M]) -> [[f32; M]; N] {
    let mut result = [[0.0; M]; N];
    for n in 0..N {
        for m in 0..M {
            result[n][m] = x[m][n];
        }
    }
    result
}

pub fn matmat_mul<const M: usize, const N: usize, const O: usize, H: TapeHolder>(
    lhs: Tensor2D<M, N, H>,
    rhs: &Tensor2D<N, O, NoTape>,
) -> Tensor2D<M, O, H> {
    let result = Tensor2D::new(matmul(lhs.data(), rhs.data()));
    let (lhs, mut tape_holder) = lhs.split_tape_holder();

    let lderiv = transpose(rhs.data());
    let rderiv = transpose(lhs.data());
    let _rhs = rhs.phantom();
    let _lhs = lhs.phantom();
    let _result = result.phantom();
    tape_holder.add_operation(move |tape| {
        let result_grad = tape.gradient(&_result);
        let d_grad_lhs = matmul(result_grad, &lderiv);
        let d_grad_rhs = matmul(&rderiv, result_grad);

        tape.mut_gradient(&_lhs).add_assign(&d_grad_lhs);
        tape.mut_gradient(&_rhs).add_assign(&d_grad_rhs);
    });

    result.with_tape_holder(tape_holder)
}

impl<const M: usize, const N: usize, const O: usize, H: TapeHolder> Mul<&Tensor2D<N, O, NoTape>>
    for Tensor2D<M, N, H>
{
    type Output = Tensor2D<M, O, H>;
    fn mul(self, rhs: &Tensor2D<N, O, NoTape>) -> Self::Output {
        matmat_mul(self, rhs)
    }
}

pub fn vecmat_mul<const N: usize, const O: usize, H: TapeHolder>(
    lhs: Tensor1D<N, H>,
    rhs: &Tensor2D<N, O, NoTape>,
) -> Tensor1D<O, H> {
    let lhs_2d = [*lhs.data(); 1];
    let result = Tensor1D::new(matmul(&lhs_2d, rhs.data())[0]);
    let (lhs, mut tape_holder) = lhs.split_tape_holder();
    todo!("update tape");
    // tape_holder.add_operation(|tape| {
    //     let lderiv = transpose(rhs.data());
    //     let rderiv = transpose(&lhs_2d);
    //     let lhs = lhs.phantom();
    //     let rhs = rhs.phantom();
    //     let result = result.phantom();
    //     tape.add_operation(|tape| {
    //         let result_grad = tape.gradient(&result);
    //         let d_grad_lhs = matmul(result_grad, &lderiv);
    //         let d_grad_rhs = matmul(&rderiv, result_grad);

    //         tape.mut_gradient(&lhs).add_assign(&d_grad_lhs);
    //         tape.mut_gradient(&rhs).add_assign(&d_grad_rhs);
    //     });
    // });
    result.with_tape_holder(tape_holder)
}

impl<const N: usize, const O: usize, H: TapeHolder> Mul<&Tensor2D<N, O, NoTape>>
    for Tensor1D<N, H>
{
    type Output = Tensor1D<O, H>;
    fn mul(self, rhs: &Tensor2D<N, O, NoTape>) -> Self::Output {
        vecmat_mul(self, rhs)
    }
}

// MxN + 1xN
pub fn broadcast_outer_add<const M: usize, const N: usize, H: TapeHolder>(
    lhs: Tensor2D<M, N, H>,
    rhs: &Tensor1D<N, NoTape>,
) -> Tensor2D<M, N, H> {
    let mut result = [[0.0; N]; M];
    for i in 0..M {
        result[i] = lhs.data()[i].add(rhs.data());
    }
    let result = Tensor2D::new(result);
    let (lhs, mut tape_holder) = lhs.split_tape_holder();
    let lhs_deriv = lhs.data().map_elems(|_| 1.0);
    let rhs_deriv = rhs.data().map_elems(|_| 1.0);
    let _rhs = rhs.phantom();
    let _lhs = lhs.phantom();
    let _result = result.phantom();
    tape_holder.add_operation(move |tape| {
        let d_grad_lhs = lhs_deriv.mul(tape.gradient(&_result));
        tape.mut_gradient(&_lhs).add_assign(&d_grad_lhs);

        // TODO test this
        let mut d_grad_rhs = [0.0; N];
        for i in 0..M {
            d_grad_rhs.add_assign(&rhs_deriv.mul(&tape.gradient(&_result)[i]));
        }
        tape.mut_gradient(&_rhs).add_assign(&d_grad_rhs);
    });
    result.with_tape_holder(tape_holder)
}

impl<const M: usize, const N: usize, H: TapeHolder> Add<&Tensor1D<N, NoTape>>
    for Tensor2D<M, N, H>
{
    type Output = Tensor2D<M, N, H>;
    fn add(self, rhs: &Tensor1D<N, NoTape>) -> Self::Output {
        broadcast_outer_add(self, rhs)
    }
}

pub fn add<T: Tensor>(lhs: &T::NoTape, rhs: T) -> T {
    let result = T::NoTape::new(lhs.data().add(rhs.data()));
    let (rhs, mut tape_holder) = rhs.split_tape_holder();
    let lhs_deriv = lhs.data().map_elems(|_| 1.0);
    let rhs_deriv = rhs.data().map_elems(|_| 1.0);
    let _lhs = lhs.phantom();
    let _rhs = rhs.phantom();
    let _result = result.phantom();
    tape_holder.add_operation(move |tape| {
        let d_grad_lhs = lhs_deriv.mul(tape.gradient(&_result));
        tape.mut_gradient(&_lhs).add_assign(&d_grad_lhs);
        let d_grad_rhs = rhs_deriv.mul(tape.gradient(&_result));
        tape.mut_gradient(&_rhs).add_assign(&d_grad_rhs);
    });
    result.with_tape_holder(tape_holder)
}

pub fn sub<T: Tensor>(lhs: &T::NoTape, rhs: T) -> T {
    let result = T::NoTape::new(lhs.data().sub(rhs.data()));
    let (rhs, mut tape_holder) = rhs.split_tape_holder();
    let lhs_deriv = lhs.data().map_elems(|_| 1.0);
    let rhs_deriv = rhs.data().map_elems(|_| -1.0);
    let _lhs = lhs.phantom();
    let _rhs = rhs.phantom();
    let _result = result.phantom();
    tape_holder.add_operation(move |tape| {
        let d_grad_lhs = lhs_deriv.mul(tape.gradient(&_result));
        tape.mut_gradient(&_lhs).add_assign(&d_grad_lhs);

        let d_grad_rhs = rhs_deriv.mul(tape.gradient(&_result));
        tape.mut_gradient(&_rhs).add_assign(&d_grad_rhs);
    });
    result.with_tape_holder(tape_holder)
}

pub fn mul<T: Tensor>(lhs: &T::NoTape, rhs: T) -> T {
    let data = lhs.data().mul(rhs.data());
    let result = T::NoTape::new(data);
    let (rhs, mut tape_holder) = rhs.split_tape_holder();
    let lhs_deriv: T::ArrayType = rhs.data().clone();
    let rhs_deriv: T::ArrayType = lhs.data().clone();
    let _lhs = lhs.phantom();
    let _rhs = rhs.phantom();
    let _result = result.phantom();
    tape_holder.add_operation(move |tape| {
        let d_grad_lhs = lhs_deriv.mul(tape.gradient(&_result));
        tape.mut_gradient(&_lhs).add_assign(&d_grad_lhs);

        let d_grad_rhs = rhs_deriv.mul(tape.gradient(&_result));
        tape.mut_gradient(&_rhs).add_assign(&d_grad_rhs);
    });
    result.with_tape_holder(tape_holder)
}

macro_rules! binary_ops_impl {
    ($typename:ident, [$($Vs:tt),*]) => {

// &T<NoTape> + T<H>
impl<$(const $Vs: usize, )* H> Add<$typename<$($Vs, )* H>> for &$typename<$($Vs, )* NoTape>
where
    H: TapeHolder
{
    type Output = $typename<$($Vs, )* H>;
    fn add(self, rhs: $typename<$($Vs, )* H>) -> Self::Output {
        add(self, rhs)
    }
}

// &T<NoTape> - T<H>
impl<$(const $Vs: usize, )* H> Sub<$typename<$($Vs, )* H>> for &$typename<$($Vs, )* NoTape>
where
    H: TapeHolder
{
    type Output = $typename<$($Vs, )* H>;
    fn sub(self, rhs: $typename<$($Vs, )* H>) -> Self::Output {
        sub(self, rhs)
    }
}

// &T<NoTape> * T<H>
impl<$(const $Vs: usize, )* H> Mul<$typename<$($Vs, )* H>> for &$typename<$($Vs, )* NoTape>
where
    H: TapeHolder
{
    type Output = $typename<$($Vs, )* H>;
    fn mul(self, rhs: $typename<$($Vs, )* H>) -> Self::Output {
        mul(self, rhs)
    }
}
    };
}

binary_ops_impl!(Tensor0D, []);
binary_ops_impl!(Tensor1D, [N]);
binary_ops_impl!(Tensor2D, [M, N]);
binary_ops_impl!(Tensor3D, [M, N, O]);
binary_ops_impl!(Tensor4D, [M, N, O, P]);

macro_rules! broadcast_sub_impl {
    ($typename:ident, [$($Vs:tt),*], $rhsty:ident, [$($Zs:tt),*]) => {
impl<$(const $Vs: usize, )* H: TapeHolder> std::ops::Sub<&$rhsty<$($Zs, )* NoTape>> for $typename<$($Vs, )* H> {
    type Output = Self;
    fn sub(self, rhs: &$rhsty<$($Zs, )* NoTape>) -> Self::Output {
        let result = <Self::Output as Tensor>::NoTape::new(self.data().sub(rhs.data()));
        let (lhs, mut tape_holder) = self.split_tape_holder();
        let lhs_deriv = lhs.data().map_elems(|_| 1.0);
        let rhs_deriv = rhs.data().map_elems(|_| -1.0);
        let _lhs = lhs.phantom();
        let _rhs = rhs.phantom();
        let _result = result.phantom();
        tape_holder.add_operation(move |tape| {
            let d_grad_lhs = lhs_deriv.mul(tape.gradient(&_result));
            tape.mut_gradient(&_lhs).add_assign(&d_grad_lhs);

            // TODO test this
            let d_grad_rhs = tape.gradient(&_result).mul(&rhs_deriv).reduce_inner(&|x, y| x + y);
            tape.mut_gradient(&_rhs).add_assign(&d_grad_rhs);
        });
        result.with_tape_holder(tape_holder)
    }
}
    };
}

broadcast_sub_impl!(Tensor1D, [M], Tensor0D, []);
broadcast_sub_impl!(Tensor2D, [M, N], Tensor1D, [M]);
broadcast_sub_impl!(Tensor3D, [M, N, O], Tensor2D, [M, N]);
broadcast_sub_impl!(Tensor4D, [M, N, O, P], Tensor3D, [M, N, O]);

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_broadcast_sub_1d() {
//         let a: Tensor1D<3> = Tensor1D::new(arr1(&[1.0, 2.0, 3.0]));
//         let b: Tensor1D<1> = Tensor1D::new(arr1(&[1.0]));
//         let r = a.trace() - &b;
//         assert_eq!(r.data(), arr1(&[0.0, 1.0, 2.0]));
//         let gradients = backward(r.mean());
//         assert_eq!(
//             gradients
//                 .gradient_for(a.id())
//                 .clone()
//                 .to_shape((3,))
//                 .unwrap(),
//             arr1(&[1.0 / 3.0; 3])
//         );
//         assert_eq!(
//             gradients
//                 .gradient_for(b.id())
//                 .clone()
//                 .to_shape((1,))
//                 .unwrap(),
//             arr1(&[-1.0; 1])
//         );
//     }

//     #[test]
//     fn test_broadcast_sub_2d() {
//         let a: Tensor2D<2, 3> = Tensor2D::new(arr2(&[[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]]));
//         let b: Tensor2D<2, 1> = Tensor2D::new(arr2(&[[1.0], [2.0]]));
//         // let r = broadcast_sub_2d(a.trace(), &b);
//         let r = a.trace() - &b;
//         assert_eq!(r.data(), arr2(&[[0.0, 1.0, 2.0], [2.0, 3.0, 4.0]]));
//         let gradients = backward(r.mean());
//         assert_eq!(
//             gradients
//                 .gradient_for(a.id())
//                 .clone()
//                 .to_shape((2, 3))
//                 .unwrap(),
//             arr2(&[[1.0 / 6.0; 3]; 2])
//         );
//         assert_eq!(
//             gradients
//                 .gradient_for(b.id())
//                 .clone()
//                 .to_shape((2, 1))
//                 .unwrap(),
//             arr2(&[[-0.5; 1]; 2])
//         );
//     }
// }
