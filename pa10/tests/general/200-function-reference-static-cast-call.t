bool eq_char(char lhs, char rhs) { return lhs == rhs; }

int main() {
  bool (&fn)(char, char) = eq_char;
  return static_cast<bool (&)(char, char)>(fn)(':', ':') ? 0 : 1;
}
