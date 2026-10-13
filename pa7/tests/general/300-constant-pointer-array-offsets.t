int array[3];
int matrix[2][3];
constexpr int* end = array + 3;
constexpr int* previous = end - 1;
constexpr bool same = previous == &2[array];
constexpr long distance = end - array;
constexpr long reverse = array - end;
constexpr bool ordered = array < end;
constexpr bool row = matrix[1] + 2 == &matrix[1][2];
int result()
{
  return same + distance + reverse + ordered + row;
}
