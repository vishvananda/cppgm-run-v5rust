// N3485 focus: 14.7.2 [temp.explicit] explicit instantiation
template<class T>
void f(T);

extern template void f<int>(int);
