struct Outer {
  struct Token {};
  struct Buffer {
    Buffer(Token);
  };
};

Outer::Buffer::Buffer(Token) {}

int main() {
  Outer::Token t;
  Outer::Buffer b(t);
  return 0;
}
