// VALIDATION: compile-pass
// N3485 focus: 15.4 [except.spec] paragraphs 5 and 8; 15.3 [except.handle] paragraph 3.
// Declaration restrictions only; no throwing function body or EH control.

namespace control_0
{
struct B{virtual void f() throw(int,double);};struct D:B{void f() throw(int);};
}

namespace control_1
{
struct B{virtual void f() throw(int);};struct D:B{void f() noexcept;};
}

namespace control_2
{
struct B{virtual void f() throw();};struct D:B{void f() noexcept;};
}

namespace control_3
{
struct B{virtual void f() throw(const int,int,double);};struct D:B{void f() throw(int,int);};
}

namespace control_4
{
struct E{};struct F:E{};struct B{virtual void f() throw(E);};struct D:B{void f() throw(F);};
}

namespace control_5
{
struct E{};struct F:E{};struct B{virtual void f() throw(const E*);};struct D:B{void f() throw(F*);};
}

namespace control_6
{
struct E{};struct B{virtual void f() throw(const void*);};struct D:B{void f() throw(E*);};
}

namespace control_7
{
struct B{virtual void f() throw(int*);};struct D:B{void f() throw(decltype(nullptr));};
}

namespace control_8
{
struct B{virtual void f() throw(const int* const*);};struct D:B{void f() throw(int**);};
}

namespace control_9
{
struct B{virtual ~B() noexcept;};struct D:B{};
}


// Omitted and defaulted destructor specifications retain the finite base list.
namespace finite_destructors
{
  struct Base { virtual ~Base() throw(int); };
  struct Implicit : Base {};
  struct Defaulted : Base { ~Defaulted() = default; };
  struct Provided : Base { ~Provided() {} };
  struct Nothrow : Base { ~Nothrow() noexcept; };
}

// Const pointer references do permit the standard catch-matching conversions.
namespace const_pointer_reference
{
  struct Base { virtual void f() throw(const int* const&); };
  struct Derived : Base { void f() throw(int*); };
}

int main() { return 0; }
