// lowir.md storage=readonly: a statically initialized const object of a class
// states storage=readonly when the class, at any depth (an element's, a base's,
// a member's), has no mutable or volatile member and no non-trivial destructor:
// a mutable member may change in a const object (7.1.6.1p4) and a destructor
// may write one (12.4p2).
struct Pair { int first; int second; };
struct Counter { int value; mutable int reads; };
struct HoldsCounter { Counter inner; int extra; };
struct FromCounter : Counter { int extra; };
struct Device { volatile int status; };
struct Logged { int value; ~Logged() {} };
struct Outer { Pair pair; int tail; };

const Pair pair_object = {1, 2};
const Pair pair_array[2] = {{3, 4}, {5, 6}};
const Outer nested_object = {{7, 8}, 9};
const Counter mutable_member = {10, 0};
const HoldsCounter mutable_in_member = {{11, 0}, 12};
const FromCounter mutable_in_base = {};
const Device volatile_member = {13};
const Logged destroyed = {14};

int main()
{
  mutable_member.reads = 1;
  int sum = pair_object.second + pair_array[1].first + nested_object.pair.second +
    nested_object.tail + mutable_member.value + mutable_member.reads + mutable_in_member.extra +
    mutable_in_base.extra + volatile_member.status + destroyed.value;
  return sum == 2 + 5 + 8 + 9 + 10 + 1 + 12 + 0 + 13 + 14 ? 0 : 1;
}
