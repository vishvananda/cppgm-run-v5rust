namespace std {
class type_info { public: bool operator==(const type_info&) const; bool operator!=(const type_info&) const; };
}

template<class T>
int same_typeid() { return typeid(T) == typeid(T); }

int main() { return same_typeid<int>(); }
