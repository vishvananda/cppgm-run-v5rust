// VALIDATION: compile-pass
// N3485 focus: 5.2.2 [expr.call], 9.3.1 [class.mfct.non-static]

struct Text {
  int value;
  Text(int v) : value(v) {}
};

struct Stream {
  Text text() const & { return Text(7); }
  Text text() && { return Text(9); }
  void text(const Text &) {}
};

int main()
{
  Stream stream;
  Text value = true ? stream.text() : Text(1);
  return value.value == 7 ? 0 : 1;
}
