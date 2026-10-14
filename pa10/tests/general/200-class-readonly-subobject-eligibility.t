struct Ordinary { int value; };
struct Mutable { mutable int value; };
struct Volatile { volatile int value; };
struct Nested { Volatile members[2]; int value; };
const Ordinary ordinary = {1};
const Mutable changeable = {2};
const Volatile observable = {3};
const Nested nested = {{{4}, {5}}, 6};
volatile int target = 7;
struct Reference { volatile int& value; };
struct Pointer { volatile int* value; };
const Reference reference = {target};
const Pointer pointer = {&target};
int main() {
  changeable.value = 8;
  return ordinary.value + changeable.value + observable.value +
    nested.members[1].value + reference.value + *pointer.value;
}
