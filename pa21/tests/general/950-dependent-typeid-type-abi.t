namespace std{class type_info{public:virtual ~type_info();};}
template<class T>auto reflect(T)->decltype(typeid(T)){return typeid(T);}
int main(){reflect(1);return 0;}
