struct Iter {
  int pos;

  Iter operator-(int n) const {
    Iter result;
    result.pos = pos - n;
    return result;
  }
};

int operator-(const Iter & left, const Iter & right) {
  return left.pos - right.pos;
}

template<class T>
T identity(T value) {
  return value;
}

int main() {
  Iter first;
  first.pos = 5;
  Iter last;
  last.pos = 3;
  int diff = identity(first - last);
  return diff == 2 ? 0 : 1;
}
