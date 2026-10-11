// VALIDATION: compile-pass
// N3485 focus: 11 [class.access], 14.5.4 [temp.friend]

template<class T>
class direct_heap
{
    struct impl
    {
        struct dispatcher
        {
        };
    };

    typedef typename impl::dispatcher dispatcher;

    template<class U>
    friend class direct_wrapper;
};

template<class T>
class direct_wrapper
{
    typedef direct_heap<T> q_type;

public:
    struct iterator : q_type::dispatcher
    {
        int read()
        {
            return 7;
        }
    };
};

struct inherited_base
{
    typedef int inherited_type;
};

template<class T>
class inherited_heap : private inherited_base
{
    template<class U>
    friend class inherited_wrapper;
};

template<class T>
class inherited_wrapper
{
    typedef inherited_heap<T> q_type;

public:
    struct iterator
    {
        typedef typename q_type::inherited_type inherited_type;

        inherited_type value;

        int read()
        {
            value = 5;
            return value;
        }
    };
};

template<class T>
int check()
{
    typename direct_wrapper<T>::iterator direct;
    typename inherited_wrapper<T>::iterator inherited;
    return direct.read() + inherited.read();
}

int main()
{
    return check<int>() == 12 ? 0 : 1;
}
