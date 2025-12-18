#include <format>
#include <generated_page_types.h>
#include "headers/macros.h"
#include "headers/object.h"
#include <shared/reflected_objects.h>

using std::format;

bool Object::is_pair() const
{
    return this->m_pair.has_value();
}

bool Object::has_children() const
{
    return !this->m_children.empty();
}

bool Object::is_first_in_pair() const
{
    DEBUG_ASSERT(this->is_pair(), format("Attempted to check object pair order (in reference to obj {})", this->name()));
    return this->m_is_first_in_pair;
}

bool Object::is_second_in_pair() const
{
    DEBUG_ASSERT(this->is_pair(), format("Attempted to check object pair order (in reference to obj {})", this->name()));
    return !this->m_is_first_in_pair;
}

const std::optional<std::weak_ptr<Object>> &Object::parent() const
{
    return this->m_parent;
}

const std::vector<std::shared_ptr<Object>> &Object::children() const
{
    DEBUG_ASSERT(this->has_children(), format("Attempted to access children of an object without any. (in reference to obj {})", this->name()));
    return this->m_children;
}

const std::string &Object::name() const
{
    return this->m_name;
}

std::optional<KnownObject> Object::get_pair_rep() const
{
    return this->m_pair_rep;
}

std::shared_ptr<Object> ObjectFactory::node_to_obj(const Node &node)
{

    KnownObject rep = page_type_from_string(node.name);
    std::shared_ptr<Object> inner_obj = std::make_shared<Object>(Object(node.name, rep));

    return inner_obj;
}

std::shared_ptr<Object> ObjectFactory::create()
{
    Node root = obj[0];
    std::shared_ptr<Object> root_obj = node_to_obj(root);
    construct_children(root, root_obj);

    return root_obj;
}

void ObjectFactory::construct_children(const Node &parent_node, std::shared_ptr<Object> parent)
{

    for (int i = 0; i < parent_node.children.size(); i++)
    {
        Node current_child = parent_node.children[i];
        std::shared_ptr<Object> current_child_obj = node_to_obj(current_child);

        current_child_obj->m_parent = std::weak_ptr(parent);
        parent->m_children.emplace_back(current_child_obj);
        construct_children(current_child, current_child_obj);

        if (current_child.pair.has_value())
        {
            DEBUG_ASSERT(parent_node.children.size() >= i + 1, "Attempted to access object pair when parent of both objects doesn't contain enough children!");
            Node next_child_pair = parent_node.children[i + 1];
            std::shared_ptr<Object> next_child_pair_object = node_to_obj(next_child_pair);
            next_child_pair_object->m_pair = std::weak_ptr(current_child_obj);
            next_child_pair_object->m_pair_rep = current_child_obj->representitive;
            next_child_pair_object->m_is_first_in_pair = false;
            next_child_pair_object->m_parent = std::weak_ptr(parent);

            current_child_obj->m_pair = std::weak_ptr(next_child_pair_object);
            current_child_obj->m_pair_rep = next_child_pair_object->representitive;
            current_child_obj->m_is_first_in_pair = true;

            parent->m_children.emplace_back(next_child_pair_object);

            construct_children(next_child_pair, next_child_pair_object);
            i += 1; // skip next since we just constructed it
        }
    }
}
