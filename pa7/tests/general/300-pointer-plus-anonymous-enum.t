char* f(char* p) {
  enum { CHUNK = 16 };
  return p + CHUNK;
}
