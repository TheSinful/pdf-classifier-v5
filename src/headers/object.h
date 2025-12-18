#pragma once

#include <string>
#include <vector>
#include <memory>
#include <optional>
#include <map>
#include <unordered_map>
#include <shared/reflected_objects.h>
#include <shared/generated_page_types.h>

class Object
{
private:
    friend class ObjectFactory;

    std::string m_name;
    std::vector<std::shared_ptr<Object>> m_children;
    std::optional<std::weak_ptr<Object>> m_parent;
    std::optional<std::weak_ptr<Object>> m_pair;
    std::optional<KnownObject> m_pair_rep;
    bool m_is_first_in_pair;

    Object(std::string name, KnownObject rep);

public:
    /// @brief The enum representation of "this", which should be passed around and can be easily mapped back to "this"
    const KnownObject representitive;

    bool has_children() const;
    bool is_pair() const;
    bool is_first_in_pair() const;
    bool is_second_in_pair() const;
    std::optional<KnownObject> get_pair_rep() const;

    const std::optional<std::weak_ptr<Object>> &parent() const;
    const std::vector<std::shared_ptr<Object>> &children() const;
    const std::string &name() const;
};

class ObjectFactory
{
private:
    std::shared_ptr<Object> node_to_obj(const Node &node);
    void construct_children(const Node &parent_node, std::shared_ptr<Object> parent);

public:
    std::shared_ptr<Object> create();
    

};
