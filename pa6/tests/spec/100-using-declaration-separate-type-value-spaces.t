// N3485 focus: 7.3.3 [namespace.udecl] introduces type and value names independently.
struct Base { int item; int call(); };
struct Derived : Base {
  typedef long item;
  typedef char call;
  using Base::item;
  using Base::call;
};
namespace Types { typedef int item; typedef long call; }
namespace Values { int item; int call(); }
namespace Both {
  using Types::item;
  using Types::call;
  using Values::item;
  using Values::call;
}
