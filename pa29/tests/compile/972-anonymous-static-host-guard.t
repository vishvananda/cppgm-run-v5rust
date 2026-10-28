int next();
int selected() {
  static union { int value = next(); };
  return value;
}
