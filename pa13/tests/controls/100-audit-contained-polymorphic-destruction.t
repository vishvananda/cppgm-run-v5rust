// Inertness distinguishes an object's own polymorphic destruction from its
// contained subobjects, while retaining observable destructor bodies; expect exit 0
int count;
struct B { virtual ~B() {} };
struct Holder { B member; };
struct Observable { B member; ~Observable() { ++count; } };
int main() {
  { Holder holder; Observable observable; }
  return count != 1;
}
