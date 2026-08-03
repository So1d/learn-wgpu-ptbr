use flume::bounded;
use wgpu::util::{BufferInitDescriptor, DeviceExt};

pub async fn run() -> anyhow::Result<()> {
    let instance = wgpu::Instance::default();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .unwrap();
    let (device, queue) = adapter.request_device(&Default::default()).await.unwrap();

    let shader = device.create_shader_module(wgpu::include_wgsl!("introduction.wgsl"));

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Introduction Compute Pipeline"),
        layout: None,
        module: &shader,
        entry_point: None,
        compilation_options: Default::default(),
        cache: Default::default(),
    });

    let input_data = (0..10_000u32).collect::<Vec<_>>();

    let input_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("input"),
        contents: bytemuck::cast_slice(&input_data),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
    });

    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("output"),
        size: input_buffer.size(),
        usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::STORAGE,
        mapped_at_creation: false,
    });

    let temp_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("temp"),
        size: input_buffer.size(),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: input_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: output_buffer.as_entire_binding(),
            },
        ],
    });

    let mut encoder = device.create_command_encoder(&Default::default());

    {
        // Especificamos 64 threads por workgroup no shader, então precisamos calcular quantos
        // workgroups precisamos despachar.
        let num_dispatches = input_data.len().div_ceil(64) as u32;

        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(num_dispatches, 1, 1);
    }

    encoder.copy_buffer_to_buffer(&output_buffer, 0, &temp_buffer, 0, output_buffer.size());

    queue.submit([encoder.finish()]);

    {
        // O processo de mapeamento é assíncrono, então precisaremos criar um canal para obter
        // a flag de sucesso do nosso mapeamento
        let (tx, rx) = bounded(1);

        // Enviamos o sucesso ou falha do nosso mapeamento via callback
        temp_buffer.map_async(wgpu::MapMode::Read, .., move |result| {
            tx.send(result).unwrap()
        });

        // O callback que enviamos para map_async só será chamado após o
        // device ser consultado (poll) ou a queue ser enviada
        device.poll(wgpu::PollType::wait_indefinitely())?;

        // Verificamos aqui se o mapeamento foi bem-sucedido
        rx.recv_async().await??;

        // Em seguida, obtemos os bytes que estavam armazenados no buffer
        let output_data = temp_buffer.get_mapped_range(..)?;

        // Agora que temos os dados na CPU, podemos fazer o que quisermos com eles
        assert_eq!(&input_data, bytemuck::cast_slice(&output_data));
    }

    // Precisamos desmapear o buffer para poder usá-lo novamente
    temp_buffer.unmap();

    log::info!("Success!");

    Ok(())
}
