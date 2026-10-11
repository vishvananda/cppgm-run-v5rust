struct Stream {
  int mask;

  Stream& operator<<(bool) {
    mask = mask * 10 + 1;
    return *this;
  }

  Stream& operator<<(const void*) {
    mask = mask * 10 + 2;
    return *this;
  }

  Stream& operator<<(Stream& (*pf)(Stream&)) {
    return pf(*this);
  }
};

Stream& operator<<(Stream& stream, char) {
  stream.mask = stream.mask * 10 + 3;
  return stream;
}

Stream& operator<<(Stream& stream, const char*) {
  stream.mask = stream.mask * 10 + 4;
  return stream;
}

Stream& manip(Stream& stream) {
  stream.mask = stream.mask * 10 + 5;
  return stream;
}

int main() {
  Stream stream;
  char ch = 'x';
  const char *text = "text";
  stream.mask = 0;
  stream << ch << "hi" << text << manip;
  return stream.mask == 3445 ? 0 : 1;
}
