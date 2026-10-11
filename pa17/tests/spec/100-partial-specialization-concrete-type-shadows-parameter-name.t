// VALIDATION: compile-pass
// N3485 focus: 14.5.5 [temp.class.spec]
// A partial specialization parameter can have the same spelling as a concrete
// type used by a later, distinct partial specialization.

template<class T>
struct action {};

template<class Act, class A>
struct trait {
  typedef char type;
};

template<class Act, class A>
struct trait<action<Act>, A> {
  typedef A type;
};

struct A {};

struct B {
  char data[2];
};

template<class Act>
struct trait<action<Act>, A> {
  typedef B type;
};

typedef trait<action<int>, A>::type selected_type;

static_assert(sizeof(selected_type) == sizeof(B), "concrete type partial selected");

int main()
{
  return sizeof(selected_type) == sizeof(B) ? 0 : 1;
}
