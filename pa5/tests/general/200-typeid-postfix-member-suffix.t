namespace std {
class type_info {
public:
  const char *name() const;
};
}

int main() {
  return typeid(int).name()[0];
}
