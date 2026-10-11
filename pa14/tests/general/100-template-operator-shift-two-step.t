struct Stream {
  int count;
};

template<class T>
Stream& operator<<(Stream& stream, const T&) {
  stream.count = stream.count + 1;
  return stream;
}

int main() {
  Stream stream;
  stream.count = 0;
  Stream* p = &(stream << 1 << 2);
  return p == &stream && stream.count == 2 ? 0 : 1;
}
