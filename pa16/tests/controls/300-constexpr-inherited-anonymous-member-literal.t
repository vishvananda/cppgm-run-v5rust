// An inherited constructor checks anonymous member initializers at their
// inline positions, including whether a typed leaf is constant. Expect exit 0.
int runtime();
struct Base { constexpr Base(int) {} };
struct Nonconstant : Base {
    using Base::Base;
    union { int value = runtime(); };
};
struct Constant : Base {
    using Base::Base;
    union { int value = 4; };
};
static_assert(!__is_literal_type(Nonconstant), "nonconstant anonymous initializer");
static_assert(__is_literal_type(Constant), "constant anonymous initializer");
int main() { return 0; }
