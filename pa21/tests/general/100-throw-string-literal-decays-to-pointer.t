int main() {
  try { throw "x"; }
  catch(char const *p) { return *p != 'x'; }
}
