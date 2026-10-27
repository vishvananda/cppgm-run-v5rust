// Only a complete object visits its shared virtual bases. Base zeroing uses
// the nonvirtual alignment; static null ranges retain stable overlap clipping.
struct Target { int value; };
struct Holder { int Target::*member; };
struct Left : virtual Holder {};
struct Right : virtual Holder {};
struct Complete : Left, Right { int value; };
Complete global;
Complete array[2];
struct alignas(16) Aligned : Holder {};
struct Base : virtual Aligned { char value; };
struct Derived : Base { Derived() : Base() {} };
Derived aligned;
int main() {
  Complete local = Complete();
  Derived base;
  return global.member != nullptr || array[0].member != nullptr ||
         array[1].member != nullptr || local.member != nullptr;
}
