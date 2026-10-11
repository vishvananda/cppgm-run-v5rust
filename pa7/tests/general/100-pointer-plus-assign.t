// HHC-169
const char* f(const char* p, int n) {
  p += n;
  return p;
}
