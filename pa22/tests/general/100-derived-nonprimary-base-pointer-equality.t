// N3485 focus: 5.10 [expr.eq], 4.10 [conv.ptr], 10 [class.derived]

struct left_base {
  int left;
};

struct right_base {
  int right;
};

struct derived : left_base, right_base {
  int value;
};

int main() {
  derived object;
  derived * derived_pointer = &object;
  right_base * base_pointer = &object;
  if(derived_pointer != base_pointer) {
    return 1;
  }
  if(base_pointer != derived_pointer) {
    return 2;
  }

  derived const * const_derived_pointer = &object;
  right_base volatile * volatile_base_pointer = &object;
  if(!(const_derived_pointer == volatile_base_pointer)) {
    return 3;
  }

  derived_pointer = 0;
  base_pointer = 0;
  return derived_pointer == base_pointer ? 0 : 4;
}
