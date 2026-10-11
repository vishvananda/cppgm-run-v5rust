struct index {
  operator long() const { return 1; }
};
int main() { int values[2] = { 0, 7 }; return values[index()] == 7 ? 0 : 1; }
