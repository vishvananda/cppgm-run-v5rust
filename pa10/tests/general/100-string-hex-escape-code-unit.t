int main() {
  if (sizeof("\xab") != 2) {
    return 1;
  }
  if ((unsigned char)"\xab"[0] != 0xab) {
    return 2;
  }
  return 0;
}
