pub struct Builder<'lifetime_3> {
    entries: Vec<wgpu::BindGroupEntry<'lifetime_3>>,
    layout: Option<&'lifetime_3 wgpu::BindGroupLayout>,
    device: &'lifetime_3 wgpu::Device
}

impl<'lifetime_3> Builder<'lifetime_3> {
    pub fn new(device: &'lifetime_3 wgpu::Device) -> Self {
        Builder {
            entries: Vec::new(),
            layout: None,
            device: device
        }
    }

    fn reset(&mut self) {
        self.entries.clear();
    }

    pub fn set_layout(&mut self, layout: &'lifetime_3 wgpu::BindGroupLayout) {
        self.layout = Some(layout);
    }

    pub fn add_material(&mut self, view: &'lifetime_3 wgpu::TextureView, sampler: &'lifetime_3 wgpu::Sampler) {
        self.entries.push(wgpu::BindGroupEntry {
            binding: self.entries.len() as u32,
            resource: wgpu::BindingResource::TextureView(view)
        });

        self.entries.push(wgpu::BindGroupEntry {
            binding: self.entries.len() as u32,
            resource: wgpu::BindingResource::Sampler(sampler)
        });
    }

    pub fn build(&mut self, label: &str) -> wgpu::BindGroup {
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout: self.layout.unwrap(),
            entries: &self.entries
        });

        self.reset();

        bind_group
    }
}
