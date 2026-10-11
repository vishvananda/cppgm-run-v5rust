struct Stream {
  int count;

  Stream& operator<<(unsigned long) {
    count = count + 1;
    return *this;
  }
};

struct Token {};

Stream& operator<<(Stream& stream, const Token&) {
  stream.count = stream.count + 1;
  return stream;
}

#define S2(x) x << token << 1ul
#define S4(x) S2(S2(x))
#define S8(x) S4(S4(x))
#define S16(x) S8(S8(x))
#define S32(x) S16(S16(x))
#define S64(x) S32(S32(x))
#define S128(x) S64(S64(x))
#define S256(x) S128(S128(x))

int main() {
  Stream stream;
  Token token;
  stream.count = 0;
  Stream* p = &(S256(stream));
  return p == &stream && stream.count == 256 ? 0 : 1;
}
