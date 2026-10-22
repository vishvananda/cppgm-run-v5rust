template<class T> struct ErrorType
{
	T value;
	void unused() { T::missing(); }
};
void permitted() throw(ErrorType<int>) {}
int main()
{
	permitted();
	return 0;
}
