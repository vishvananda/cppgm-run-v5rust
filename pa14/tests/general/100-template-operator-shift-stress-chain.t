struct Stream {
  int count;
};

struct Token {};

template<class T>
Stream& operator<<(Stream& stream, const T&) {
  stream.count = stream.count + 1;
  return stream;
}

int main() {
  Stream stream;
  Token token;
  stream.count = 0;
  Stream* p =
      &(stream << token << token << token << token << token << token << token
               << token << token << token << token << token << token << token
               << token << token);
  return p == &stream && stream.count == 16 ? 0 : 1;
}
