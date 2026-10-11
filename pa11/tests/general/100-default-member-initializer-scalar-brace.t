struct Cache {
  int hash_code{};
  int * before{};
};

int main() {
  Cache cache;
  return cache.hash_code == 0 && cache.before == 0 ? 0 : 1;
}
