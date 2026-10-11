// VALIDATION: compile-pass
// N3485 focus: 14.8.2.5 [temp.deduct.type]

template<class T, int N>
void translate(T (&)[N][N], T (&)[N - 1])
{
}

int main()
{
  int matrix[3][3] = {};
  int vector[2] = {};
  translate(matrix, vector);
}
