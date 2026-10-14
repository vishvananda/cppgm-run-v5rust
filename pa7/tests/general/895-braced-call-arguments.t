int empty(int n) { return n; }
int scalar(int n) { return n; }
int scalar(const long& n) { return n; }
int array(const int (&n)[2]) { return n[0] + n[1]; }
int array(const long (&n)[2]) { return n[0] + n[1]; }
int main() {
  int (*indirect)(const int (&)[2]) = array;
  return scalar({3}) + empty({}) + array({1, 2}) + indirect({3, 4});
}
