// An array new-expression with a braced-init-list copy-initializes its first
// elements from the clauses and value-initializes the rest (N3485 8.5.4p3,
// 5.3.4p17); an empty list value-initializes every element. A runtime count
// is checked against the number of clauses before allocating (5.3.4p7).
int count(int value) { return value; }

int* listed(int elements) { return new int[elements]{1, 2, 3}; }

int main() {
  int status = 0;
  int* values = listed(count(5));
  if (values[0] != 1 || values[2] != 3 || values[3] != 0 || values[4] != 0)
    status |= 1;
  delete[] values;
  long* zeroed = new long[count(3)]{};
  if (zeroed[0] != 0 || zeroed[2] != 0) status |= 2;
  delete[] zeroed;
  int (*rows)[2] = new int[count(3)][2]{{1, 2}, {3, 4}};
  if (rows[1][1] != 4 || rows[2][0] != 0 || rows[2][1] != 0) status |= 4;
  delete[] rows;
  char* exact = new char[3]{'a', 'b', 'c'};
  if (exact[0] != 'a' || exact[2] != 'c') status |= 8;
  delete[] exact;
  return status;
}
