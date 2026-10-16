// expect exit 0
int destroyed;
struct Item { int value; };
struct Owner { ~Owner() { ++destroyed; } };
Item make(Owner&) { return Item(); }
int main() {
  Owner owner;
  const Item& reference = make(owner);
  return reference.value + make(owner).value + make(owner).value;
}
