int main() {
  const char data[2] = { 1, 0 };
  char& r = const_cast<char&>(data[0]);
  return r == 1 ? 0 : 1;
}
