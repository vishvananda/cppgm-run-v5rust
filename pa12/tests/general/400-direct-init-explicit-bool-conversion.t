struct S {
  explicit operator bool() const { return true; }
};

int main()
{
  S s;
  bool b(s);
  bool c = bool(s);
  return b && c ? 0 : 1;
}
