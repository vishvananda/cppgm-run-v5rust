// expect exit 0
int effects;
int next() { return ++effects; }

struct Initial {
  int first = next();
  struct {
    int x = next();
    union { int y = next(); long other; };
  };
  int last = next();
};

Initial global;

struct Written {
  int first;
  struct { int x; int values[3]; };
  union { int choice; long other; };
  Written() : other(7), values{3, 4, 5}, x(2), first(1) {}
  Written(int) : other(17), values{13, 14, 15}, x(12), first(11) {}
};

union DefaultChoice {
  struct { int x = 3; int y = 4; };
  long other;
};

union Choice {
  struct { int x = next(); int y = next(); };
  long other;
  Choice() : other(9) {}
};

struct Reference {
  char prefix;
  struct { int& value; };
  Reference(int& v) : prefix(1), value(v) {}
};

#pragma pack(push, 1)
struct Bits {
  char prefix;
  struct { unsigned a : 3; unsigned long b : 63; };
  char last;
  Bits() : prefix(1), a(3), b(65), last(7) {}
};
#pragma pack(pop)

int alive;
struct Item {
  int value;
  Item(int v) : value(v) { ++alive; }
  ~Item() { --alive; }
};

struct Lifetime {
  char prefix = 1;
  struct { Item first{3}; };
  struct { Item second{4}; };
};

struct ManualLifetime {
  union { Item object; int integer; };
  ManualLifetime() : object(8) {}
  ~ManualLifetime() { object.~Item(); }
};

int main() {
  if (global.first != 1 || global.x != 2 || global.y != 3 || global.last != 4) return 1;
  if (effects != 4) return 2;
  effects = 0;
  Initial local;
  if (local.first != 1 || local.x != 2 || local.y != 3 || local.last != 4) return 3;
  if (effects != 4) return 4;

  Written written;
  if (written.first != 1 || written.x != 2 || written.other != 7) return 5;
  if (written.values[0] != 3 || written.values[1] != 4 || written.values[2] != 5) return 6;
  Written another(0);
  if (another.first != 11 || another.x != 12 || another.other != 17) return 15;
  if (another.values[0] != 13 || another.values[1] != 14 || another.values[2] != 15) return 16;
  DefaultChoice defaults;
  if (defaults.x != 3 || defaults.y != 4) return 7;
  Choice chosen;
  if (chosen.other != 9 || effects != 4) return 8;

  int value = 11;
  Reference reference(value);
  reference.value = 12;
  if (value != 12 || reference.prefix != 1) return 9;
  Bits bits;
  if (bits.prefix != 1 || bits.a != 3 || bits.b != 65 || bits.last != 7) return 10;

  {
    Lifetime lifetime;
    if (alive != 2 || lifetime.prefix != 1) return 11;
    if (lifetime.first.value != 3 || lifetime.second.value != 4) return 12;
  }
  if (alive != 0) return 13;
  {
    ManualLifetime lifetime;
    if (alive != 1 || lifetime.object.value != 8) return 14;
  }
  return alive;
}
