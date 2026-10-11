// N3485 [except.handle]: a miss searches dynamically enclosing handlers.
int main() {
  try {
    try {
      try { throw 7; }
      catch (bool) { return 1; }
    } catch (long) { return 2; }
  } catch (int value) { return value != 7; }
  return 3;
}
