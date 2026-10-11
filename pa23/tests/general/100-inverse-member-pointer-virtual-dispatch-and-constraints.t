struct Padding { int prefix; };
struct Base { int value; };
struct Derived : Padding, Base {
  virtual int read() { return prefix; }
};
int invoke(Base* object, int (Base::*member)()) { return (object->*member)(); }

struct Virtual : virtual Base { int member; };
struct rejected { char bytes[2]; };
template<class T> T make();
template<class T> auto inverse(int)
  -> decltype(static_cast<int Base::*>(make<int T::*>()), char());
template<class> rejected inverse(...);
template<class T> auto forward(int)
  -> decltype(static_cast<int T::*>(make<int Base::*>()), char());
template<class> rejected forward(...);
static_assert(sizeof(inverse<Virtual>(0)) == sizeof(rejected),
              "virtual inverse must fail substitution");
static_assert(sizeof(forward<Virtual>(0)) == sizeof(rejected),
              "virtual forward must fail substitution");

int main() {
  Derived object;
  object.prefix = 7;
  int (Base::*member)() = static_cast<int (Base::*)()>(&Derived::read);
  return invoke(&object, member) != 7;
}
