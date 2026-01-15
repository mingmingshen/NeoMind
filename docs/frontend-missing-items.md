# 前端重构遗漏项清单

## 根据原始计划 (`docs/architecture/device-refactor-complete.md`)

### ✅ 已完成
1. ✅ 清理类型定义：移除 `uplink`/`downlink` 遗留字段
2. ✅ 更新组件：移除所有向后兼容检查
3. ✅ 更新翻译文件：术语更新
4. ✅ 设备列表已显示适配器列（表头存在）

### ❌ 未完成（计划中的内容）

#### 1. **模板选择对话框** (`TemplateSelectDialog.tsx`)
**状态**: ❌ 未实现
**位置**: `web/src/components/devices/TemplateSelectDialog.tsx`（新文件）
**计划内容**:
- 两步式对话框（选择模板 → 填写参数）
- 按分类显示模板（sensor, switch, light, energy）
- 动态表单根据模板参数生成
- 调用 `/api/devices/templates/:id/create` API

**依赖**: 需要后端先实现模板 API：
- `GET /api/devices/templates` - 获取所有模板
- `GET /api/devices/templates/:id` - 获取单个模板
- `POST /api/devices/templates/:id/create` - 从模板创建设备

#### 2. **设备列表显示适配器类型**
**状态**: ⚠️ 部分完成
**问题**: 
- ✅ 表头已有 "适配器" 列（第182行）
- ❌ 但显示的是 `plugin_name` 而不是 `adapter_type`（第203行）
- 应该显示 `device.adapter_type || 'mqtt'` 并用 Badge 显示

**需要修改**: `web/src/pages/devices/DeviceList.tsx` 第203行

#### 3. **设备列表添加模板入口按钮**
**状态**: ❌ 未实现
**计划内容**:
- 在设备列表工具栏添加"模板添加"按钮
- 保留现有的"手动添加"按钮作为次要选项
- 按钮布局：
  ```tsx
  <Button onClick={() => setTemplateDialogOpen(true)}>
    <Plus /> {t('devices:add.title')}
  </Button>
  <Button variant="outline" onClick={() => setAddDeviceDialogOpen(true)}>
    <FileJson /> {t('devices:add.manual')}
  </Button>
  ```

**需要修改**: `web/src/pages/devices/DeviceList.tsx` 和 `web/src/pages/devices.tsx`

#### 4. **API 调用方法**
**状态**: ❌ 未实现
**需要添加**: `web/src/lib/api.ts`
```typescript
getDeviceTemplates: () => fetchAPI<{ templates: Template[] }>('/devices/templates'),
getDeviceTemplate: (id: string) => fetchAPI<Template>(`/devices/templates/${id}`),
createDeviceFromTemplate: (id: string, params: Record<string, string>) =>
  fetchAPI<{ device_id: string }>(`/devices/templates/${id}/create`, {
    method: 'POST',
    body: JSON.stringify({ params }),
  }),
```

## 实施优先级

### 高优先级（立即可以完成）
1. ✅ **修复设备列表适配器显示** - 只需要改一行代码
   - 将 `device.plugin_name` 改为 `device.adapter_type || 'mqtt'`
   - 添加 Badge 组件显示

2. ✅ **添加模板入口按钮** - 简单 UI 修改
   - 在 DeviceList 工具栏添加按钮（即使后端未实现模板 API）

### 中优先级（需要后端支持）
3. ⏭️ **实现模板选择对话框** - 需要后端 API 先实现
   - 等待后端实现 `/api/devices/templates` 相关端点
   - 或先实现 UI，使用 mock 数据

4. ⏭️ **添加 API 调用方法** - 依赖后端 API
   - 如果后端还没实现，可以先添加方法但会报错

## 后端依赖检查

需要检查后端是否实现了：
- [ ] `GET /api/devices/templates` 
- [ ] `GET /api/devices/templates/:id`
- [ ] `POST /api/devices/templates/:id/create`

如果后端未实现，可以：
1. 先完成 UI 部分（使用 mock 数据或禁用）
2. 等待后端实现后再启用
3. 或者标记为"未来功能"
