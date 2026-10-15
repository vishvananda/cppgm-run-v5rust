// expect exit 0
int alive;
struct Item {
  int value;
  Item() : value(3) { ++alive; }
  ~Item() { --alive; }
};

struct { int unused() { return 1; } } unused;
struct { Item item; } global;
namespace {
struct { Item item; } private_global;
}

typedef struct { Item item; } Adopted;
typedef struct { struct { Item item; }; } Nested;

int first() {
  struct { Item item; } object;
  return object.item.value != 3;
}
int second() {
  struct { Item item; } object;
  return object.item.value != 3;
}
inline int shared() {
  enum { unused_enum };
  typedef struct { int unused; } Earlier;
  struct { Item item; } object;
  return object.item.value != 3;
}
int read(decltype(global)& object) { return object.item.value; }

int main() {
  if (alive != 2 || read(global) != 3 || private_global.item.value != 3) return 1;
  if (first() || second() || shared() || alive != 2) return 2;
  {
    Adopted adopted;
    Nested nested;
    struct { Item item; } objects[2];
    if (alive != 6) return 3;
    if (adopted.item.value != 3 || nested.item.value != 3) return 4;
    if (objects[0].item.value != 3 || objects[1].item.value != 3) return 5;
  }
  return alive != 2;
}
