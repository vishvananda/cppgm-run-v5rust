// lowir.md storage=readonly: a global definition states storage=readonly when
// it is statically initialized, not thread-local, and of a const type that is
// not volatile (an array's qualifiers are its element type's, 3.9.3p5): const
// scalars, const pointers and const arrays of them do; a volatile or a
// non-const object does not. The thread-local and dynamically initialized
// cases need PA11 and are in its 400-readonly-dynamic-and-thread-local-globals.
const int scalar = 3;
const int table[4] = {1, 2, 3, 4};
const int matrix[2][2] = {{5, 6}, {7, 8}};
const char* const names[2] = {"a", "bc"};
int* const fixed_pointer = 0;
const char* movable_pointer = "d";
const volatile int volatile_scalar = 9;
const int single_table[1] = {10};
int writable_table[2] = {12, 13};

int main()
{
  int sum = scalar + table[3] + matrix[1][1] + names[1][1] + (fixed_pointer ? 1 : 0) +
    movable_pointer[0] + volatile_scalar + single_table[0] + writable_table[1];
  return sum == 3 + 4 + 8 + 'c' + 0 + 'd' + 9 + 10 + 13 ? 0 : 1;
}
