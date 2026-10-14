typedef int Pair[2];
int read(const int (&r)[2]) { return r[0] + r[1]; }
int main() {
  const int (&nested)[2][2]{{1, 2}, {3}};
  const double (&floating)[2]{1, 2.5};
  const int (&functional)[2] = Pair{4, 5};
  int (&&mutable_array)[2]{6, 7};
  mutable_array[0] = 8;
  return nested[1][1] + floating[0] + functional[0] +
    mutable_array[0] + read({9, 10}) + Pair{11, 12}[0];
}
