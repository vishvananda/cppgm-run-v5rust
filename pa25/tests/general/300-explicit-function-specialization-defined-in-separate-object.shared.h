struct payload {};

template<class T>
int explicit_function_specialization(int);

template<>
int explicit_function_specialization<payload>(int);
