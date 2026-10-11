namespace std {
class type_info { public: bool operator==(const type_info&) const; bool operator!=(const type_info&) const; };
}

int main() {
  return typeid(int) == typeid(int);
}
