use mint::IntoMint;

pub mod units {
    use paste::paste;

    macro_rules! euclid_units {
        ($($unit:ident => $default:ty),+) => {
            paste! {
                $(
                    pub struct [<$unit Unit>]; 

                    pub mod [<$unit:snake>] {
                        pub type Point2D<T = $default> = euclid::Point2D<T, super::[<$unit Unit>]>;
                        pub type Vector2D<T = $default> = euclid::Vector2D<T, super::[<$unit Unit>]>;
                        pub type Box2D<T = $default> = euclid::Box2D<T, super::[<$unit Unit>]>;
                        pub type Size2D<T = $default> = euclid::Size2D<T, super::[<$unit Unit>]>;
                        pub type Rect<T = $default> = euclid::Rect<T, super::[<$unit Unit>]>;
                    }
    
                )+
            }
        };
    }
    
    euclid_units!(World => f32, Screen => f32, Map => u32);
}

pub trait IntoMintExt {
    fn minto<C>(self) -> C
    where
        Self: Sized,
        Self: IntoMint,
        C: From<<Self as IntoMint>::MintType>;
}

impl<T> IntoMintExt for T
where
    T: IntoMint,
{
    fn minto<C>(self) -> C
    where
        C: From<<Self as IntoMint>::MintType>,
    {
        C::from(self.into())
    }
}
