int array[3];
constexpr int* pointer = array;
int*const& pointer_reference = pointer;
constexpr int& element_reference = array[1];
const int& temporary = 7;
const int object = 3;
constexpr int value = *&object;
constexpr bool same = pointer_reference == array;
constexpr bool element = &element_reference == array + 1;
constexpr int extended = temporary;
int result()
{
  return value + same + element + extended;
}
