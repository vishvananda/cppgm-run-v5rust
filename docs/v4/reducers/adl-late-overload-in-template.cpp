template <bool B, class T = void> struct enable_if {};
template <class T> struct enable_if<true, T> { typedef T type; };
template <class T> struct is_boxed { static const bool value = false; };

namespace N {
// The generic overload, excluded by SFINAE for a boxed argument.
template <class T>
typename enable_if<!is_boxed<T>::value>::type act(T&, T&) {}
}

template <class A> struct Box;
template <class A> struct is_boxed<Box<A> > { static const bool value = true; };

template <class A>
struct Box
{
	A first;
	void act(Box& other)
	{
		using N::act;
		act(first, other.first);
	}
};

namespace N {
// The boxed overload, declared after the using-declaration: only argument
// dependent lookup at instantiation can find it.
template <class A> void act(Box<A>& a, Box<A>& b) { a.act(b); }
}

int main() { Box<Box<int> > outer, other; outer.act(other); return 0; }
