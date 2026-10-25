// expect exit 0
template<class T> int sum(T& values) {
  int result = 0;
  for (auto value : values) {
    if (value == 2) continue;
    switch (value) {
      case 4: continue;
      default: break;
    }
    if (value == 5) break;
    result += value;
  }
  for (auto value : values) {
    for (auto inner : values) {
      if (inner == 1) continue;
      break;
    }
    result += value;
    break;
  }
  return result;
}
template<class T> int dormant(T& values) {
  for (auto value : values) {
    if (value) continue;
    break;
  }
  return 0;
}
int main() {
  int values[5] = {1, 2, 3, 4, 5};
  return sum(values) == 5 ? 0 : 1;
}
