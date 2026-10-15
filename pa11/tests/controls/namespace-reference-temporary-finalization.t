// N3485 12.2p5, 3.6.3p1: declaration-time reference temporaries persist
// until shutdown; only completed objects are destroyed, in reverse order.
// expect exit 0
int alive;
int destroyed;
struct Watch {
  ~Watch() {
    // Fail at shutdown, after main, if a cleanup was omitted or reordered.
    if (alive != 0 || destroyed != 976548321) *(volatile int*)0 = 1;
  }
} watch;
struct Object {
  int id;
  Object(int value) : id(value) { ++alive; }
  ~Object() { --alive; destroyed = destroyed * 10 + id; }
};
const Object& first = Object(1);
struct Pair {
  const Object& first;
  const Object& second;
  ~Pair() { destroyed = destroyed * 10 + 8; }
} pair = {Object(2), Object(3)};
struct Holder { const Object& object; } holders[2] = {{Object(4)}, {Object(5)}};
const Object (&objects)[2] = {{6}, {7}};
bool choose() { return false; }
const Object& selected = choose() ? static_cast<const Object&&>(Object(8))
                                  : static_cast<const Object&&>(Object(9));
struct Owner { Object member = Object(0); } owner;
int main() {
  return alive != 9 || destroyed != 0 || first.id != 1 ||
         pair.second.id != 3 || holders[1].object.id != 5 ||
         objects[1].id != 7 || selected.id != 9 || owner.member.id != 0;
}
