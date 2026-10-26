namespace std{class type_info{public:virtual ~type_info();};}
template<class T>auto reflect(T value)->decltype(typeid(value)){return typeid(value);}int main(){reflect(1);return 0;}
