// VALIDATION: compile-pass
// N3485 focus: 12.2 [class.temporary]

struct Text {
  Text();
  Text(const char *);
  Text & operator=(const Text &);
  ~Text();
};

bool operator==(const Text &, const char *);

struct Box {
  Text val;
  Box();
  ~Box();
  Text get() const;
};

int main()
{
  Box box;
  box.val = Text("ok");
  return box.get() == "ok" ? 0 : 1;
}
