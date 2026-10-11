constexpr int square(int n) { return n * n; }
constexpr int seed = square(2);

int main()
{
  int first[3] = {seed, square(3)};
  int second[3] = {seed, square(3)};
  first[1] = 11;
  return first != second && first[1] == 11 && second[0] == 4 &&
         second[1] == 9 && second[2] == 0 ? 0 : 1;
}
