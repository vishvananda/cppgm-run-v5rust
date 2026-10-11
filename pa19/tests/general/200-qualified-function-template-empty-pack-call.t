namespace N {
template<class... Args>
int make(Args&&... args) {
  return 0;
}
}

int main() {
  return N::make();
}
