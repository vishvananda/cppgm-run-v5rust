// expect exit 0
// N3485 focus: 5.4p5, 5.2.9p2/p11 and 14.8.2: invalid virtual
// downcasts fail substitution, including bases of virtual bases.
struct Base {};
struct Plain : Base {};
struct Left : private virtual Base {};
struct Right : virtual Base {};
struct Diamond : Left, Right {};
struct Mixed : Left, Plain {};
struct Virtual : virtual Plain {};
struct Indirect : Virtual {};
template<class T> struct Dependent : virtual T {
    void unused() { T::missing(); }
};
template<class... T> struct Packed : virtual T... {};

template<class T, class U, class = decltype((U*)((T*)0))>
constexpr bool pointer_cast(int) { return true; }
template<class, class> constexpr bool pointer_cast(...) { return false; }
template<class T, class U, class = decltype(static_cast<U&>(*(T*)0))>
constexpr bool reference_cast(int) { return true; }
template<class, class> constexpr bool reference_cast(...) { return false; }
template<class T, class U, class = decltype(static_cast<U*>((T*)0))>
constexpr bool static_pointer_cast(int) { return true; }
template<class, class> constexpr bool static_pointer_cast(...) { return false; }

static_assert(!pointer_cast<Base, Diamond>(0), "shared virtual downcast");
static_assert(!pointer_cast<Base, Mixed>(0), "mixed subobjects");
static_assert(!pointer_cast<Base, Indirect>(0), "base of a virtual base");
static_assert(!pointer_cast<Base, Dependent<Base>>(0), "complete target on demand");
static_assert(!pointer_cast<Base, Packed<Base>>(0), "expanded virtual edge");
static_assert(!reference_cast<Base, Diamond>(0), "reference downcast");
static_assert(!reference_cast<Base, Indirect>(0), "indirect reference downcast");
static_assert(!reference_cast<Base, Dependent<Base>>(0), "dependent reference target");
static_assert(static_pointer_cast<Diamond, Base>(0), "shared accessible route");
static_assert(!static_pointer_cast<Mixed, Base>(0), "ambiguous upcast");
static_assert(!static_pointer_cast<Left, Base>(0), "inaccessible upcast");
static_assert(pointer_cast<Left, Base>(0), "C-style access exception");
static_assert(pointer_cast<Base, Plain>(0), "ordinary downcast");
int main() { return 0; }
