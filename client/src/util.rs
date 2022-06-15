// Here is a macro that does the job:
macro_rules! product {
    ($first:ident, $($next:ident),*) => (
        $first.iter() $(
            .flat_map(|e| std::iter::repeat(e)
                .zip($next.iter()))
        )*
    );
}