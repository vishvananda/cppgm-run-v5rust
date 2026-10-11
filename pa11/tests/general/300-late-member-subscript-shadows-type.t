struct rank {
};

struct container {
  int storage[4];

  int &operator[](int i) {
    return storage[i];
  }
};

struct sets {
  void make_set(int x) {
    rank[x] = 0;
  }

  container rank;
};

int main() {
  sets s;
  s.rank[1] = 5;
  s.make_set(1);
  return s.rank[1];
}
