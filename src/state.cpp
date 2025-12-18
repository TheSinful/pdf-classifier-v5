#include <macros.h>
#include "state.h"

const std::map<KnownObject, std::shared_ptr<Object>> &State::window(int start, int end) const
{
    std::map<KnownObject, std::shared_ptr<Object>> result = {};

    DEBUG_ASSERT(end >= start, "Window end was greater than window start!");
    DEBUG_ASSERT(start < 0, "Start of window was less than 0!");
    DEBUG_ASSERT(end <= m_current_classified.size(), "End of the windo was greater than current_classified.");

    for (int i = start; i < end; i++)
    {
        KnownObject i_ident = m_current_classified[i];
        std::shared_ptr<Object> i_obj = lookup_ident(i_ident);

        result.insert(std::make_pair(i_ident, i_obj));
    }

    return result;
}

std::shared_ptr<Object> State::lookup_ident(KnownObject ident) const
{
    auto iter = m_known_objs.find(ident);

    if (iter != m_known_objs.end())
    {
        return iter->second;
    }
    else
    {
        throw std::runtime_error(std::format("Failed to find corresponding Object within known_objs, with provided identifier: {}", page_type_to_string(ident)));
    }
}

std::shared_ptr<Object> State::current_considering_obj() const
{
    return lookup_ident(m_considering);
}

std::shared_ptr<Object> State::previous_classified_obj() const
{
    return lookup_ident(previous_classified_ident());
}

const KnownObject &State::previous_classified_ident() const
{
    return m_current_classified.back();
}

State::State() 
{
    ObjectFactory factory;
    std::shared_ptr<Object> root = factory.create();
    std::shared_ptr<Object> next_child = root->ch; 
    


    m_known_objs = ; 
    m_current_classified = {}; 
}