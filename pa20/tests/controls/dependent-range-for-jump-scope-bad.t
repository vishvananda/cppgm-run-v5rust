// expect reject
template<class T> void outside(T& values) {
  for (auto value : values) {
    if (value) continue;
    break;
  }
  continue;
}
int main() { return 0; }
