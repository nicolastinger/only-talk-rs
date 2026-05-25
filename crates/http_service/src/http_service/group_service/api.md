# Group Service 请求接口文档

## 基础信息

| 项目 | 说明 |
|---|---|
| **基础路径** | `/group/chat` |
| **认证方式** | 请求头中携带用户 UUID（由 `get_uuid_from_header!` 宏从 HTTP Header 中提取） |
| **统一响应格式** | `CommonResponse<T>` |

### 统一响应结构

```json
{
  "code": 200,
  "data": T,
  "message": "Success"
}
```

| 字段 | 类型 | 说明 |
|---|---|---|
| `code` | u32 | 200=成功, 500=失败 |
| `data` | T | 具体业务数据 |
| `message` | String | 描述信息 |

### 角色常量说明

| 常量 | 值 | 说明 |
|---|---|---|
| `ROLE_MEMBER` | 0 | 普通成员 |
| `ROLE_ADMIN` | 1 | 管理员 |
| `ROLE_OWNER` | 2 | 群主 |

### 成员状态常量

| 常量 | 值 | 说明 |
|---|---|---|
| `STATUS_NORMAL` | 1 | 正常 |
| `STATUS_QUIT` | 2 | 已退出 |
| `STATUS_KICKED` | 3 | 已被移除 |

### 邀请状态常量

| 常量 | 值 | 说明 |
|---|---|---|
| `INVITATION_PENDING` | 1 | 待处理 |
| `INVITATION_ACCEPTED` | 2 | 已接受 |
| `INVITATION_DECLINED` | 3 | 已拒绝 |

### 群组状态常量

| 值 | 说明 |
|---|---|
| 1 | 正常 |
| 2 | 已解散 |

---

## 1. 创建群组

**`POST /group/chat/create`**

创建一个新的群组，创建者自动成为群主（Owner）。

### 请求体

```json
{
  "group_name": "我的群聊",
  "avatar": "https://...",
  "description": "群描述",
  "max_members": 500
}
```

| 字段 | 类型 | 必填 | 校验规则 | 说明 |
|---|---|---|---|---|
| `group_name` | String | ✅ | 长度 1-100 | 群名称 |
| `avatar` | Option\<String\> | ❌ | 最长 500 | 群头像 URL |
| `description` | Option\<String\> | ❌ | 最长 500 | 群描述 |
| `max_members` | Option\<i32\> | ❌ | - | 最大成员数，默认 500 |

### 响应数据 `GroupInfoVO`

```json
{
  "code": 200,
  "data": {
    "group_uuid": "uuid-string",
    "group_name": "我的群聊",
    "avatar": "https://...",
    "owner_uuid": "创建者uuid",
    "description": "群描述",
    "max_members": 500,
    "member_count": 1,
    "created_at": 1700000000000,
    "updated_at": 1700000000000,
    "status": 1
  },
  "message": "Success"
}
```

| 字段 | 类型 | 说明 |
|---|---|---|
| `group_uuid` | String | 群组唯一标识 |
| `group_name` | String | 群名称 |
| `avatar` | Option\<String\> | 群头像 URL |
| `owner_uuid` | String | 群主 UUID |
| `description` | Option\<String\> | 群描述 |
| `max_members` | i32 | 最大成员数 |
| `member_count` | i64 | 当前成员数 |
| `created_at` | i64 | 创建时间（毫秒时间戳） |
| `updated_at` | i64 | 更新时间（毫秒时间戳） |
| `status` | i16 | 群状态（1=正常） |

---

## 2. 获取群组信息

**`GET /group/chat/info/{group_uuid}`**

根据群 UUID 查询群组详细信息（含成员数）。

### 路径参数

| 参数 | 类型 | 说明 |
|---|---|---|
| `group_uuid` | String | 群组 UUID |

### 响应数据 `Option<GroupInfoVO>`

成功时返回 `GroupInfoVO`（结构同上），群不存在时返回 `null`。

---

## 3. 更新群组信息

**`PUT /group/chat/update`**

更新群组的基本信息。**仅群主（Owner）可操作**，非群主调用返回 `false`。

### 请求体

```json
{
  "group_uuid": "uuid-string",
  "group_name": "新群名",
  "avatar": "https://...",
  "description": "新描述"
}
```

| 字段 | 类型 | 必填 | 校验规则 | 说明 |
|---|---|---|---|---|
| `group_uuid` | String | ✅ | - | 要更新的群 UUID |
| `group_name` | Option\<String\> | ❌ | 长度 1-100 | 新群名称 |
| `avatar` | Option\<String\> | ❌ | 最长 500 | 新群头像 URL |
| `description` | Option\<String\> | ❌ | 最长 500 | 新群描述 |

> 传入的 `None` 字段不会覆盖原有值（采用 `dto.xxx.or(existing)` 策略）。

### 响应数据 `bool`

```json
{ "code": 200, "data": true, "message": "Success" }
```

| 值 | 说明 |
|---|---|
| `true` | 更新成功 |
| `false` | 更新失败（非群主或群不存在） |

---

## 4. 解散群组

**`DELETE /group/chat/dissolve/{group_uuid}`**

解散指定群组。**仅群主（Owner）可操作**，将群状态设为 `2`（已解散）。

### 路径参数

| 参数 | 类型 | 说明 |
|---|---|---|
| `group_uuid` | String | 要解散的群 UUID |

### 响应数据 `bool`

| 值 | 说明 |
|---|---|
| `true` | 解散成功 |
| `false` | 解散失败（非群主或群不存在） |

---

## 5. 获取我加入的群列表

**`GET /group/chat/my/list`**

查询当前用户加入的所有群组列表。

### 请求参数

无（从请求头获取用户 UUID）

### 响应数据 `Vec<GroupListItemVO>`

```json
{
  "code": 200,
  "data": [
    {
      "group_uuid": "uuid-string",
      "group_name": "群名",
      "avatar": "https://...",
      "owner_uuid": "群主uuid",
      "member_count": 10,
      "last_msg_time": null,
      "unread_count": 0
    }
  ],
  "message": "Success"
}
```

| 字段 | 类型 | 说明 |
|---|---|---|
| `group_uuid` | String | 群组 UUID |
| `group_name` | String | 群名称 |
| `avatar` | Option\<String\> | 群头像 |
| `owner_uuid` | String | 群主 UUID |
| `member_count` | i64 | 成员数 |
| `last_msg_time` | Option\<i64\> | 最后一条消息时间（当前返回 `null`） |
| `unread_count` | i64 | 未读消息数（当前返回 `0`） |

---

## 6. 获取群成员列表

**`GET /group/chat/member/list/{group_uuid}`**

查询指定群组的成员列表。

### 路径参数

| 参数 | 类型 | 说明 |
|---|---|---|
| `group_uuid` | String | 群组 UUID |

### 响应数据 `Vec<GroupMemberVO>`

```json
{
  "code": 200,
  "data": [
    {
      "user_uuid": "uuid-string",
      "role": 2,
      "nickname": "昵称",
      "join_time": 1700000000000,
      "muted": false,
      "status": 1
    }
  ],
  "message": "Success"
}
```

| 字段 | 类型 | 说明 |
|---|---|---|
| `user_uuid` | String | 用户 UUID |
| `role` | i16 | 角色：0=成员, 1=管理员, 2=群主 |
| `nickname` | Option\<String\> | 群内昵称 |
| `join_time` | i64 | 加入时间（毫秒时间戳） |
| `muted` | bool | 是否禁言 |
| `status` | i16 | 状态：1=正常, 2=已退出, 3=已移除 |

---

## 7. 邀请成员入群

**`POST /group/chat/member/invite`**

邀请用户加入群组。**需要管理员（Admin）及以上权限**，被邀请人将收到系统通知。

### 请求体

```json
{
  "group_uuid": "uuid-string",
  "user_uuids": ["uuid1", "uuid2"]
}
```

| 字段 | 类型 | 必填 | 校验规则 | 说明 |
|---|---|---|---|---|
| `group_uuid` | String | ✅ | - | 群 UUID |
| `user_uuids` | Vec\<String\> | ✅ | 长度 ≥ 1 | 被邀请人 UUID 列表 |

### 业务逻辑

1. 校验操作者是否为群成员且角色 ≥ Admin（`role >= 1`）
2. 跳过已是群成员的用户
3. 跳过已有待处理邀请的用户
4. 对已有非待处理邀请的用户，更新其邀请状态为待处理
5. 对全新用户创建邀请记录
6. 向每位被邀请人发送系统通知：`"邀请你加入群聊「{群名}」"`

### 响应数据 `Vec<String>`

返回实际成功发送邀请的用户 UUID 列表（跳过的不会出现在列表中）。

---

## 8. 接受入群邀请

**`POST /group/chat/member/invite/accept`**

接受群组邀请，加入群组。仅待处理（`PENDING`）状态的邀请可被接受。

### 请求体

```json
{
  "group_uuid": "uuid-string"
}
```

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `group_uuid` | String | ✅ | 要加入的群 UUID |

### 业务逻辑

1. 查找当前用户在该群的待处理邀请
2. 更新邀请状态为 `INVITATION_ACCEPTED`（2）
3. 将用户添加为群成员（角色为 `ROLE_MEMBER`）
4. 清除 Redis 中该群的成员缓存（`group:members:{group_uuid}`）
5. 向邀请人发送通知：`"用户已接受加入群聊「{群名}」的邀请"`

### 响应数据 `bool`

| 值 | 说明 |
|---|---|
| `true` | 接受成功 |
| `false` | 接受失败（无待处理邀请） |

---

## 9. 拒绝入群邀请

**`POST /group/chat/member/invite/decline`**

拒绝群组邀请。仅待处理（`PENDING`）状态的邀请可被拒绝。

### 请求体

```json
{
  "group_uuid": "uuid-string"
}
```

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `group_uuid` | String | ✅ | 要拒绝的群 UUID |

### 业务逻辑

1. 查找当前用户在该群的待处理邀请
2. 更新邀请状态为 `INVITATION_DECLINED`（3）

### 响应数据 `bool`

| 值 | 说明 |
|---|---|
| `true` | 拒绝成功 |
| `false` | 拒绝失败（无待处理邀请） |

---

## 10. 获取待处理邀请列表

**`GET /group/chat/member/invite/pending`**

查询当前用户所有待处理的入群邀请。

### 请求参数

无（从请求头获取用户 UUID）

### 响应数据 `Vec<GroupInvitationVO>`

```json
{
  "code": 200,
  "data": [
    {
      "id": 1,
      "group_uuid": "uuid-string",
      "group_name": "群名",
      "group_avatar": "https://...",
      "inviter_uuid": "邀请人uuid",
      "invitee_uuid": "被邀请人uuid",
      "status": 1,
      "created_at": 1700000000000
    }
  ],
  "message": "Success"
}
```

| 字段 | 类型 | 说明 |
|---|---|---|
| `id` | i64 | 邀请记录 ID |
| `group_uuid` | String | 群组 UUID |
| `group_name` | String | 群名称 |
| `group_avatar` | Option\<String\> | 群头像 |
| `inviter_uuid` | String | 邀请人 UUID |
| `invitee_uuid` | String | 被邀请人 UUID |
| `status` | i16 | 邀请状态：1=待处理, 2=已接受, 3=已拒绝 |
| `created_at` | i64 | 创建时间（毫秒时间戳） |

---

## 11. 移除群成员

**`DELETE /group/chat/member/remove/{group_uuid}/{user_uuid}`**

将指定成员移出群组。**需要管理员（Admin）及以上权限**，且不能移除同级或更高级角色的成员。

### 路径参数

| 参数 | 类型 | 说明 |
|---|---|---|
| `group_uuid` | String | 群组 UUID |
| `user_uuid` | String | 要移除的用户 UUID |

### 业务逻辑

1. 校验操作者角色 ≥ Admin（`role >= 1`）
2. 校验目标成员角色 < 操作者角色（不能移除同级或更高级）
3. 将目标成员状态设为 `STATUS_KICKED`（3）
4. 清除 Redis 中该群的成员缓存

### 响应数据 `bool`

| 值 | 说明 |
|---|---|
| `true` | 移除成功 |
| `false` | 移除失败（权限不足、目标角色过高、操作者非成员或目标不存在） |

---

## 12. 退出群组

**`POST /group/chat/member/quit/{group_uuid}`**

退出指定群组。**群主（Owner）不能退出**，需先转让群主或解散群组。

### 路径参数

| 参数 | 类型 | 说明 |
|---|---|---|
| `group_uuid` | String | 要退出的群 UUID |

### 业务逻辑

1. 校验当前用户是否为群成员
2. 校验当前用户不是群主（`role != 2`）
3. 将成员状态设为 `STATUS_QUIT`（2）
4. 清除 Redis 中该群的成员缓存

### 响应数据 `bool`

| 值 | 说明 |
|---|---|
| `true` | 退出成功 |
| `false` | 退出失败（群主不能退出或非群成员） |

---

## 13. 设置成员角色

**`PUT /group/chat/member/set_role`**

设置群成员的角色。**仅群主（Owner）可操作**。

### 请求体

```json
{
  "group_uuid": "uuid-string",
  "user_uuid": "target-uuid",
  "role": 1
}
```

| 字段 | 类型 | 必填 | 校验规则 | 说明 |
|---|---|---|---|---|
| `group_uuid` | String | ✅ | - | 群 UUID |
| `user_uuid` | String | ✅ | - | 目标成员 UUID |
| `role` | i16 | ✅ | 范围 0-2 | 新角色：0=成员, 1=管理员, 2=群主 |

### 业务逻辑

1. 校验操作者是群主（`role == 2`）
2. 查找目标成员
3. 更新目标成员的角色

### 响应数据 `bool`

| 值 | 说明 |
|---|---|
| `true` | 设置成功 |
| `false` | 设置失败（非群主或目标成员不存在） |

---

## 14. 获取群消息历史

**`GET /group/chat/message/history`**

分页查询群组消息历史记录。**需为群成员**才能查看。

### 查询参数

| 参数 | 类型 | 必填 | 校验规则 | 说明 |
|---|---|---|---|---|
| `group_uuid` | String | ✅ | - | 群 UUID |
| `start` | Option\<u32\> | ❌ | ≥ 0，默认 0 | 起始偏移量 |
| `size` | Option\<u32\> | ❌ | 1-100，默认 20 | 每页数量 |

### 请求示例

```
GET /group/chat/message/history?group_uuid=xxx&start=0&size=20
```

### 业务逻辑

1. 校验当前用户是否为群成员，非成员返回空列表
2. 根据 `start` 和 `size` 分页查询消息记录

### 响应数据 `Vec<GroupMessageVO>`

```json
{
  "code": 200,
  "data": [
    {
      "nano_id": "nano-id-string",
      "group_uuid": "uuid-string",
      "send_user": "发送者uuid",
      "timestamp": 1700000000000,
      "raw": [0],
      "msg_type": 1,
      "recalled": false
    }
  ],
  "message": "Success"
}
```

| 字段 | 类型 | 说明 |
|---|---|---|
| `nano_id` | String | 消息 Nano ID |
| `group_uuid` | String | 群组 UUID |
| `send_user` | String | 发送者 UUID |
| `timestamp` | i64 | 发送时间（毫秒时间戳） |
| `raw` | Vec\<u8\> | 消息原始内容（字节流） |
| `msg_type` | i16 | 消息类型（默认 1） |
| `recalled` | bool | 是否已撤回 |

---

## 15. 获取未读群消息数

**`GET /group/chat/message/unread`**

查询当前用户在各群组中的未读消息数量。仅返回有未读消息的群组。

### 请求参数

无（从请求头获取用户 UUID）

### 业务逻辑

1. 查询当前用户加入的所有群
2. 对每个群，根据 `last_read_msg_id` 统计未读消息数
3. 仅返回未读数 > 0 的群组

### 响应数据 `Vec<UnreadCountVO>`

```json
{
  "code": 200,
  "data": [
    {
      "group_uuid": "uuid-string",
      "unread_count": 5
    }
  ],
  "message": "Success"
}
```

| 字段 | 类型 | 说明 |
|---|---|---|
| `group_uuid` | String | 群组 UUID |
| `unread_count` | i64 | 未读消息数 |

---

## 权限矩阵总结

| 操作 | Owner (2) | Admin (1) | Member (0) |
|---|:---:|:---:|:---:|
| 创建群组 | ✅ | ✅ | ✅ |
| 获取群信息 | ✅ | ✅ | ✅ |
| 更新群信息 | ✅ | ❌ | ❌ |
| 解散群组 | ✅ | ❌ | ❌ |
| 获取我的群列表 | ✅ | ✅ | ✅ |
| 获取群成员列表 | ✅ | ✅ | ✅ |
| 邀请成员入群 | ✅ | ✅ | ❌ |
| 接受/拒绝邀请 | ✅ | ✅ | ✅ |
| 获取待处理邀请 | ✅ | ✅ | ✅ |
| 移除成员（低级） | ✅ | ✅ | ❌ |
| 退出群组 | ❌ | ✅ | ✅ |
| 设置成员角色 | ✅ | ❌ | ❌ |
| 获取消息历史 | ✅ | ✅ | ✅ |
| 获取未读消息数 | ✅ | ✅ | ✅ |

> **移除成员**的细化规则：Admin 可移除 Member，Owner 可移除 Admin 和 Member，但不能移除同级或更高级角色。
