// expect exit 0
// Injected names retain their anonymous storage offset and enclosing base.
struct Base {
  int prefix;
  union {
    int value;
    long alternate;
  };
  int read() const { return value; }
  void write(int n) { value = n; }
};

struct Derived : Base {
  int suffix;
  int read_derived() const { return value; }
  void write_derived(int n) { value = n; }
};

struct Nested {
  int prefix;
  struct {
    int middle;
    union {
      int value;
      long alternate;
    };
  };
  int read() const { return value; }
};

struct BitBase {
  int prefix;
  union {
    struct {
      signed int value : 5;
      unsigned int other : 7;
    };
    long alternate;
  };
  int read() const { return value; }
  void write(int n) { value = n; }
};

struct BitDerived : BitBase {
  void step() { ++value; }
};

Derived global_object;
int* global_field = &global_object.value;
int& global_reference = global_object.value;

struct ArrayHolder {
  int prefix;
  struct { int elements[3]; };
};

ArrayHolder global_array;
int* global_element = &global_array.elements[2];

class PrivateBase {
  union { int value; };
public:
  char tail;
  void write(int n) { value = n; }
  int read() { return value; }
};

struct PrivateDerived : PrivateBase { char extra; };

int main() {
  Derived object;
  object.prefix = 11;
  object.suffix = 12;
  object.value = 13;
  if (object.read() != 13 || object.read_derived() != 13) return 1;
  object.write(14);
  if (object.value != 14) return 2;
  object.write_derived(15);
  Derived* pointer = &object;
  if (pointer->value != 15 || pointer->Base::value != 15) return 3;
  if (object.prefix != 11 || object.suffix != 12) return 4;
  if (&object.value == &object.prefix) return 5;

  Nested nested;
  nested.prefix = 21;
  nested.middle = 22;
  nested.value = 23;
  if (nested.read() != 23 || nested.prefix != 21 || nested.middle != 22) return 6;

  BitDerived bits;
  bits.prefix = 31;
  bits.other = 32;
  bits.write(-3);
  bits.step();
  if (bits.read() != -2 || bits.value != -2 || bits.other != 32) return 7;
  if (bits.prefix != 31) return 8;

  volatile Derived observed;
  observed.prefix = 41;
  observed.value = 42;
  if (observed.value != 42 || observed.prefix != 41) return 9;

  global_object.prefix = 51;
  *global_field = 52;
  if (global_reference != 52 || global_object.value != 52) return 10;
  global_reference = 53;
  if (*global_field != 53 || global_object.prefix != 51) return 11;
  global_array.prefix = 61;
  global_array.elements[0] = 62;
  *global_element = 63;
  if (global_array.elements[2] != 63 || global_array.elements[0] != 62) return 12;
  if (global_array.prefix != 61) return 13;

  PrivateDerived packed_tail;
  packed_tail.write(71);
  packed_tail.tail = 72;
  packed_tail.extra = 73;
  if (sizeof(PrivateBase) != 8 || sizeof(PrivateDerived) != 8) return 14;
  if (packed_tail.read() != 71 || packed_tail.tail != 72 || packed_tail.extra != 73) return 15;
  return 0;
}
