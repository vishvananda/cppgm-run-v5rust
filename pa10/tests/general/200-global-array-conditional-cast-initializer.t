const char text[] = {
  static_cast<char>(true ? 'A' : 'a'),
  static_cast<char>(0)
};

int main() {
  return text[0] - 'A';
}
