// The type-shape traits libc++ spells directly instead of deriving from
// partial specializations, with the answers that distinguish them.
struct object_type {};
enum shape { one };
typedef void function_type();

// Cv-qualification belongs to the type, and a reference never carries it.
static_assert(__is_const(const int), "const int");
static_assert(!__is_const(int), "int");
static_assert(!__is_const(const int&), "reference to const");
static_assert(__is_const(int* const), "const pointer");
static_assert(!__is_const(const int*), "pointer to const");
static_assert(__is_volatile(volatile int), "volatile int");
static_assert(!__is_volatile(const int), "const is not volatile");
static_assert(!__is_volatile(volatile int&), "reference to volatile");

static_assert(__is_void(void), "void");
static_assert(__is_void(const void), "const void");
static_assert(!__is_void(void*), "pointer to void");

static_assert(__is_array(int[3]), "bounded");
static_assert(__is_array(int[]), "unbounded");
static_assert(!__is_array(int*), "pointer");
static_assert(__is_bounded_array(int[3]), "bounded array");
static_assert(!__is_bounded_array(int[]), "unbounded is not bounded");
static_assert(__is_unbounded_array(int[]), "unbounded array");
static_assert(!__is_unbounded_array(int[3]), "bounded is not unbounded");

static_assert(__is_lvalue_reference(int&), "lvalue reference");
static_assert(!__is_lvalue_reference(int&&), "rvalue reference is not lvalue");
static_assert(__is_rvalue_reference(int&&), "rvalue reference");
static_assert(!__is_rvalue_reference(int&), "lvalue reference is not rvalue");

static_assert(__is_object(int), "scalar");
static_assert(__is_object(int[3]), "array");
static_assert(__is_object(object_type), "class");
static_assert(!__is_object(int&), "reference");
static_assert(!__is_object(void), "void");
static_assert(!__is_object(function_type), "function");

static_assert(__is_arithmetic(int), "integer");
static_assert(__is_arithmetic(double), "floating");
static_assert(__is_arithmetic(bool), "bool");
static_assert(!__is_arithmetic(shape), "enum");
static_assert(!__is_arithmetic(int*), "pointer");

static_assert(__is_fundamental(int), "integer");
static_assert(__is_fundamental(void), "void");
static_assert(!__is_fundamental(int*), "pointer");
static_assert(!__is_fundamental(object_type), "class");

static_assert(__is_compound(int*), "pointer");
static_assert(__is_compound(object_type), "class");
static_assert(__is_compound(shape), "enum");
static_assert(!__is_compound(int), "integer");
static_assert(!__is_compound(void), "void");

static_assert(__is_referenceable(int), "object");
static_assert(__is_referenceable(int&), "reference");
static_assert(!__is_referenceable(void), "void");

static_assert(__is_unsigned(unsigned), "unsigned");
static_assert(__is_unsigned(bool), "bool");
static_assert(!__is_unsigned(int), "signed");
static_assert(!__is_unsigned(double), "floating");
static_assert(!__is_unsigned(int*), "pointer");

int main() { return 0; }
